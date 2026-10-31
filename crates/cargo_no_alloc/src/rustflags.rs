//! The Rust flags the checked build runs under: what the caller already has
//! (environment or Cargo config), what the checker must add, and what an
//! ambient panic strategy means for the sysroot.

use crate::cargo_config::{
    backtick_list, cargo_config_json, string_or_array, StringOrArray, Wording,
};
use crate::cli::StdMode;
use crate::outcome::refuse;
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::ffi::OsString;
use std::path::Path;

/// Added by `--immediate-abort`. `-Cpanic=immediate-abort` is still
/// unstable, hence the accompanying `-Zunstable-options`. Passing it here
/// rather than asking users to set `[profile.dev] panic = "immediate-abort"`
/// keeps the checked crate's own manifest stable-buildable: the manifest key
/// requires `cargo-features = ["panic-immediate-abort"]`, which makes the
/// whole manifest nightly-only (ADR 0002's zero-footprint rule).
pub(crate) const IMMEDIATE_ABORT_RUSTFLAGS: &[&str] =
    &["-Zunstable-options", "-Cpanic=immediate-abort"];

/// Flags every checked compilation needs. `pub` only so the driver binary
/// (which re-adds any that are missing when invoked directly) reads this very
/// list instead of keeping a copy that could drift.
pub const REQUIRED_RUSTFLAGS: &[&str] = &[
    "--cfg=no_alloc_check",
    "--check-cfg=cfg(no_alloc_check)",
    "-Zcrate-attr=feature(register_tool)",
    "-Zcrate-attr=register_tool(no_alloc_tool)",
    "-Zalways-encode-mir",
    "-Zcrate-attr=feature(rustc_attrs)",
    // `rustc_attrs` (for `#[rustc_no_mir_inline]` on markers) is an internal
    // feature; without this every checked crate warns about it.
    "-Zcrate-attr=allow(internal_features)",
    // A crate that already has `#![feature(rustc_attrs)]` would otherwise
    // fail the deny-by-default `duplicate_features` lint on the copy above.
    "-Zcrate-attr=allow(duplicate_features)",
];

/// The value of the last `-C panic=...` setting in a token stream, in the
/// order rustc would see them. `-Cpanic` is last-wins, so a stream ending in
/// `-Cpanic=unwind` compiles under `unwind` even if `immediate-abort`
/// appeared earlier — checking "does this value appear anywhere" instead of
/// "which one wins" flagged an effectively-`unwind` environment as if it
/// selected immediate-abort, which is how this was found.
///
/// Handles both the joined spelling (`-Cpanic=val`, `--codegen=panic=val`)
/// and the two-token form `rustc -C help` also documents (`-C panic=val`,
/// `--codegen panic=val`) — both reach rustc identically. A bare `-C`/
/// `--codegen` matches only the two-token form's first slot, checked before
/// the joined-prefix strip so it can't be mistaken for a (nonsensical) empty
/// joined value.
pub(crate) fn last_panic_setting<'a>(tokens: &[&'a str]) -> Option<&'a str> {
    let mut last = None;
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        if matches!(token, "-C" | "--codegen") {
            if let Some(value) = tokens
                .get(index + 1)
                .and_then(|next| next.strip_prefix("panic="))
            {
                last = Some(value);
                index += 1;
            }
        } else if let Some(rest) = token
            .strip_prefix("-C")
            .or_else(|| token.strip_prefix("--codegen"))
        {
            if let Some(value) = rest.trim_start_matches('=').strip_prefix("panic=") {
                last = Some(value);
            }
        }
        index += 1;
    }
    last
}

/// Whether the caller's own flags currently select the immediate-abort
/// panic strategy — the *last* one, since that is the one rustc uses. Split
/// out from the environment so it can be tested directly:
/// `CARGO_ENCODED_RUSTFLAGS` is `\x1f`-separated and takes precedence over
/// `RUSTFLAGS`, exactly as Cargo reads them.
pub(crate) fn declares_immediate_abort(encoded: Option<&str>, plain: Option<&str>) -> bool {
    let raw: Vec<&str> = match encoded {
        Some(encoded) => encoded.split('\x1f').collect(),
        None => plain.unwrap_or_default().split_whitespace().collect(),
    };
    // An encoded entry can itself carry the space, so flatten before pairing
    // rather than trusting the separator to have split every token.
    let tokens: Vec<&str> = raw
        .iter()
        .flat_map(|token| token.split_whitespace())
        .collect();
    last_panic_setting(&tokens) == Some("immediate-abort")
}

/// Whether the pre-flight ambient-flag guard should reject the run.
///
/// The escape hatch is `StdMode::rebuilds_std()`, not immediate-abort
/// specifically; see [`reject_ambient_immediate_abort`] for why.
pub(crate) fn ambient_immediate_abort_needs_build_std(
    std_mode: StdMode,
    ambient: Option<&AmbientRustflags>,
) -> bool {
    !std_mode.rebuilds_std() && ambient.is_some_and(AmbientRustflags::selects_immediate_abort)
}

/// The pre-flight guard. The strategy can arrive through the caller's own
/// flags instead of the checker option, and then nothing rebuilds the sysroot:
/// the crate compiles under immediate-abort while std keeps the panic runtime
/// it was precompiled with, and the report would claim `immediate_abort` for a
/// build where std's panic paths are not compiled that way at all.
///
/// The escape hatch is `StdMode::rebuilds_std()`, not the immediate-abort
/// mode: immediate-abort always rebuilds std, but the reverse isn't required.
/// A hand-set ambient strategy plus plain `--build-std` rebuilds the sysroot
/// under the same inherited flags and is exactly as coherent as the flag,
/// which `outcome::mixed_sysroot_panic_strategy` agrees with after the build.
/// Gating on immediate-abort alone would reject that legitimate combination
/// before the build runs.
pub(crate) fn reject_ambient_immediate_abort(
    report_path: &Path,
    std_mode: StdMode,
    ambient: Option<&AmbientRustflags>,
) -> Result<()> {
    if !ambient_immediate_abort_needs_build_std(std_mode, ambient) {
        return Ok(());
    }
    Err(refuse(
        report_path,
        "the environment's or Cargo config's Rust flags already select `-Cpanic=immediate-abort`, but neither \
         `--immediate-abort` nor `--build-std` was passed. That strategy only means what the \
         report says it means if the standard library is rebuilt with it too; pass \
         `--immediate-abort` (which does both) or `--build-std` (if you are setting the \
         strategy yourself)"
            .to_owned(),
    ))
}

/// The Rust flags the caller's build would use, read once so the pre-flight
/// guard and the build's own flags cannot disagree about them. `None` (at the
/// use sites) means nothing is ambient.
///
/// Cargo's precedence is `CARGO_ENCODED_RUSTFLAGS`, then `RUSTFLAGS`, then
/// configuration; the first one present (even if empty) ignores everything
/// after it, so exactly one variant is ever in play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AmbientRustflags {
    /// `\x1f`-separated, exactly as Cargo reads it.
    Encoded(String),
    /// Whitespace-separated `RUSTFLAGS`.
    Plain(String),
    /// Imported from Cargo config, which only applies when neither variable
    /// is set. Never empty: an empty import is "nothing ambient".
    FromConfig(Vec<String>),
}

impl AmbientRustflags {
    /// Reads the two variables' raw values. A non-UTF-8 value that is in
    /// effect is an error rather than being lossily decoded: the flags would
    /// then differ from what rustc sees, and the guard could miss a
    /// `-Cpanic=immediate-abort` it was written to catch. A value Cargo
    /// would ignore anyway (`RUSTFLAGS` under `CARGO_ENCODED_RUSTFLAGS`) is
    /// not inspected.
    pub(crate) fn from_env(
        encoded: Option<OsString>,
        plain: Option<OsString>,
    ) -> Result<Option<Self>> {
        let text = |name: &str, value: OsString| {
            value.into_string().map_err(|_| {
                anyhow::anyhow!(
                    "`{name}` is not valid UTF-8, so the checker cannot tell which Rust flags \
                     the shipped build uses (for one, whether they select \
                     `-Cpanic=immediate-abort`); set it to a UTF-8 value or unset it"
                )
            })
        };
        Ok(match (encoded, plain) {
            (Some(encoded), _) => Some(Self::Encoded(text("CARGO_ENCODED_RUSTFLAGS", encoded)?)),
            (None, Some(plain)) => Some(Self::Plain(text("RUSTFLAGS", plain)?)),
            (None, None) => None,
        })
    }

    pub(crate) fn from_config(flags: Vec<String>) -> Option<Self> {
        (!flags.is_empty()).then_some(Self::FromConfig(flags))
    }

    pub(crate) fn selects_immediate_abort(&self) -> bool {
        match self {
            Self::Encoded(encoded) => declares_immediate_abort(Some(encoded), None),
            Self::Plain(plain) => declares_immediate_abort(None, Some(plain)),
            // Imported flags stand in for the encoded variable: they are
            // what rustc would see, and `-Cpanic=immediate-abort` in config
            // is as much an ambient selection as one in the environment.
            Self::FromConfig(flags) => declares_immediate_abort(Some(&flags.join("\x1f")), None),
        }
    }
}

/// Cargo accepts rustflags as one whitespace-split string or as an array.
pub(crate) fn parse_rustflags_value(json: &str) -> Result<Vec<String>> {
    Ok(
        match string_or_array(
            json,
            &Wording {
                invalid_json: "Cargo config rustflags are not valid JSON",
                non_string_entry: "Cargo config rustflags contain a non-string entry",
                neither: "Cargo config rustflags are neither a string nor an array",
            },
        )? {
            StringOrArray::One(flags) => flags.split_whitespace().map(str::to_owned).collect(),
            StringOrArray::Many(flags) => flags,
        },
    )
}

/// Names of `target.'cfg(...)'` tables that set `rustflags`. Evaluating a
/// cfg expression is Cargo's job against its own target description; this
/// wrapper only needs to know whether such a table exists.
pub(crate) fn cfg_rustflags_tables(target_json: &str) -> Result<Vec<String>> {
    let target: serde_json::Value =
        serde_json::from_str(target_json).context("Cargo `target` config is not valid JSON")?;
    let tables = target
        .as_object()
        .context("Cargo `target` config is not a table")?;
    Ok(tables
        .iter()
        .filter(|(name, table)| {
            name.starts_with("cfg(")
                && table
                    .as_object()
                    .is_some_and(|t| t.contains_key("rustflags"))
        })
        .map(|(name, _)| name.clone())
        .collect())
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ConfigRustflags {
    Import(Vec<String>),
    Unsupported(String),
}

/// Cargo's precedence among configuration sources, after the environment
/// variables the caller did not set: a non-empty `target.<triple>.rustflags`
/// (together with matching `target.'cfg(..)'.rustflags`) wins outright, and
/// `build.rustflags` is consulted when the target list is absent or empty —
/// they are never merged. An empty target list does not shadow `build`.
///
/// A `cfg(...)` table is refused rather than evaluated: guessing whether it
/// matches would either drop flags Cargo applies or import ones it does not,
/// and either way the report would describe a build that is not the shipped
/// one.
pub(crate) fn select_config_rustflags(
    build: Option<Vec<String>>,
    host_target: Option<Vec<String>>,
    cfg_tables: &[String],
) -> ConfigRustflags {
    if !cfg_tables.is_empty() {
        return ConfigRustflags::Unsupported(format!(
            "Cargo config sets rustflags under {} which this checker cannot evaluate, so it \
             cannot tell which flags the shipped build uses; config rustflags keyed on \
             `cfg(...)` are not supported — put the flags in `RUSTFLAGS` instead",
            backtick_list(cfg_tables.iter().map(|name| format!("target.'{name}'")))
        ));
    }
    // Cargo falls back to `build.rustflags` when the target list is empty
    // (checked with `cargo build -v --target <host>`), so only a non-empty
    // list shadows it; `.or` would let `rustflags = []` hide real flags.
    let target = host_target.filter(|flags| !flags.is_empty());
    ConfigRustflags::Import(target.or(build).unwrap_or_default())
}

/// The rustflags plain `cargo build --target <host>` would use when the
/// caller set neither `RUSTFLAGS` nor `CARGO_ENCODED_RUSTFLAGS` (the
/// environment otherwise shadows all of config, so the caller's own flags
/// are already faithful and this is not consulted). Read through
/// `cargo config get` so the `CARGO_BUILD_RUSTFLAGS` and
/// `CARGO_TARGET_<TRIPLE>_RUSTFLAGS` spellings and `--config` overrides are
/// resolved by Cargo itself.
pub(crate) fn config_rustflags(
    cargo: &OsString,
    config_args: &[String],
    host: &str,
) -> Result<ConfigRustflags> {
    let get = |key: &str| -> Result<Option<Vec<String>>> {
        cargo_config_json(cargo, config_args, key)?
            .map(|json| parse_rustflags_value(&json))
            .transpose()
            .with_context(|| format!("while reading Cargo config key `{key}`"))
    };
    let cfg_tables = match cargo_config_json(cargo, config_args, "target")? {
        Some(json) => cfg_rustflags_tables(&json)?,
        None => Vec::new(),
    };
    Ok(select_config_rustflags(
        get("build.rustflags")?,
        get(&format!("target.{host}.rustflags"))?,
        &cfg_tables,
    ))
}

/// The `(variable, value)` the build must be given so the required flags (and
/// the panic strategy, if asked for) are in effect on top of `ambient`.
/// Pure: the caller applies it, and the same `ambient` already fed the
/// pre-flight guard, so the two cannot see different flags.
pub(crate) fn add_required_rustflags(
    ambient: Option<&AmbientRustflags>,
    immediate_abort: bool,
) -> (&'static str, String) {
    // Two passes, and the order matters. The always-required flags are added
    // only when absent, so an inherited copy is left alone. The panic
    // strategy is then appended unconditionally, because `-Cpanic` is
    // last-wins: "already present somewhere" is not the same as "in effect",
    // and an inherited `-Cpanic=immediate-abort ... -Cpanic=unwind` would
    // otherwise satisfy a presence test while the build ran under unwind,
    // with `--immediate-abort` quietly not happening. Repeating the flag is
    // harmless; being outvoted by it is not.
    let panic_strategy: &[&str] = if immediate_abort {
        IMMEDIATE_ABORT_RUSTFLAGS
    } else {
        &[]
    };
    let push = |value: &mut String, separator: char, flag: &str| {
        if !value.is_empty() {
            value.push(separator);
        }
        value.push_str(flag);
    };
    // Config flags are carried in the encoded variable, which has no
    // word-splitting, so an array entry containing a space survives intact.
    let encoded = match ambient {
        Some(AmbientRustflags::Encoded(value)) => Some(value.clone()),
        Some(AmbientRustflags::FromConfig(flags)) => Some(flags.join("\x1f")),
        Some(AmbientRustflags::Plain(_)) | None => None,
    };
    if let Some(mut value) = encoded {
        let existing: HashSet<String> = value.split('\x1f').map(str::to_owned).collect();
        for flag in REQUIRED_RUSTFLAGS {
            if !existing.contains(*flag) {
                push(&mut value, '\x1f', flag);
            }
        }
        for flag in panic_strategy {
            push(&mut value, '\x1f', flag);
        }
        ("CARGO_ENCODED_RUSTFLAGS", value)
    } else {
        let mut value = match ambient {
            Some(AmbientRustflags::Plain(value)) => value.clone(),
            _ => String::new(),
        };
        for flag in REQUIRED_RUSTFLAGS {
            if !value.split_whitespace().any(|existing| existing == *flag) {
                push(&mut value, ' ', flag);
            }
        }
        for flag in panic_strategy {
            push(&mut value, ' ', flag);
        }
        ("RUSTFLAGS", value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::strings;

    #[test]
    fn ambient_immediate_abort_is_recognized_in_either_flag_variable() {
        // Encoded takes precedence over plain, as in Cargo.
        assert!(declares_immediate_abort(
            Some("-Cpanic=immediate-abort"),
            None
        ));
        assert!(declares_immediate_abort(
            Some("--cfg=x\u{1f}-Cpanic=immediate-abort"),
            None
        ));
        assert!(declares_immediate_abort(
            None,
            Some("-Zunstable-options -Cpanic=immediate-abort")
        ));
        assert!(!declares_immediate_abort(
            Some("--cfg=x"),
            Some("-Cpanic=immediate-abort")
        ));
        // Neighbouring strategies are not this one.
        assert!(!declares_immediate_abort(None, Some("-Cpanic=abort")));
        assert!(!declares_immediate_abort(None, Some("-Cpanic=unwind")));
        assert!(!declares_immediate_abort(None, None));
    }

    /// `-C panic=val` is a documented spelling, and it reaches rustc the
    /// same way the joined one does — as two adjacent arguments in either
    /// variable, or as one encoded entry containing the space. Matching only
    /// `-Cpanic=…` let every split form through.
    #[test]
    fn ambient_immediate_abort_is_recognized_when_the_flag_is_split() {
        for plain in [
            "-Zunstable-options -C panic=immediate-abort",
            "-C panic=immediate-abort",
            "--codegen panic=immediate-abort",
            "--codegen=panic=immediate-abort",
        ] {
            assert!(declares_immediate_abort(None, Some(plain)), "{plain}");
        }
        for encoded in [
            "-Zunstable-options\u{1f}-C\u{1f}panic=immediate-abort",
            "-C panic=immediate-abort",
            "--codegen\u{1f}panic=immediate-abort",
        ] {
            assert!(declares_immediate_abort(Some(encoded), None), "{encoded}");
        }
        // A dangling `-C`, or one whose value is a different strategy, is not
        // a match — including when the next token merely looks similar.
        for plain in [
            "-C",
            "-C panic=abort",
            "-C opt-level=3 panic=immediate-abort-ish",
            "--codegen debuginfo=2",
        ] {
            assert!(!declares_immediate_abort(None, Some(plain)), "{plain}");
        }
    }

    /// `-Cpanic` is last-wins in rustc, so "the value appears somewhere" is
    /// the wrong question — only the last setting is in effect. A stream
    /// ending in `unwind` must not be reported as selecting immediate-abort
    /// just because it appears earlier, and vice versa.
    #[test]
    fn declares_immediate_abort_honors_the_last_setting_not_any_setting() {
        // Immediate-abort mentioned first, overridden by unwind: not in effect.
        assert!(!declares_immediate_abort(
            Some("-Cpanic=immediate-abort\u{1f}-Cpanic=unwind"),
            None
        ));
        assert!(!declares_immediate_abort(
            None,
            Some("-Zunstable-options -Cpanic=immediate-abort -Cpanic=unwind")
        ));
        // The reverse order: unwind first, immediate-abort wins.
        assert!(declares_immediate_abort(
            Some("-Cpanic=unwind\u{1f}-Cpanic=immediate-abort"),
            None
        ));
        // Mixed joined/split spellings, still last-wins.
        assert!(!declares_immediate_abort(
            None,
            Some("-C panic=immediate-abort -Cpanic=abort")
        ));
    }

    #[test]
    fn immediate_abort_flag_is_appended_even_when_an_equal_flag_is_inherited() {
        // `-Cpanic` is last-wins, so "already present somewhere" is not the
        // same as "in effect": an inherited pair ending in `-Cpanic=unwind`
        // must not stop the checker's own copy from being appended last.
        let ambient =
            AmbientRustflags::Encoded("-Cpanic=immediate-abort\u{1f}-Cpanic=unwind".into());
        let (name, outvoted) = add_required_rustflags(Some(&ambient), true);
        assert_eq!(name, "CARGO_ENCODED_RUSTFLAGS");
        let last = outvoted.split('\u{1f}').next_back().unwrap_or_default();
        assert_eq!(last, "-Cpanic=immediate-abort", "in `{outvoted}`");
    }

    #[test]
    fn ambient_guard_accepts_build_std_without_the_immediate_abort_flag() {
        // The exact case this round found: `--build-std` alone, no
        // `--immediate-abort`, ambient flags already selecting the
        // strategy — coherent, and must not be pre-flight rejected.
        let ambient = AmbientRustflags::Encoded("-Cpanic=immediate-abort".into());
        assert!(!ambient_immediate_abort_needs_build_std(
            StdMode::BuildStd,
            Some(&ambient)
        ));
        // Neither flag: still a real mix, still rejected.
        assert!(ambient_immediate_abort_needs_build_std(
            StdMode::Precompiled,
            Some(&ambient)
        ));
        // No ambient strategy at all: nothing to reject regardless of
        // `--build-std`.
        for mode in [StdMode::Precompiled, StdMode::BuildStd] {
            assert!(!ambient_immediate_abort_needs_build_std(mode, None));
        }
    }

    /// The guard and the build read one value, whichever source supplied it:
    /// each variant must both trip the guard and have its flags carried into
    /// the variable the build is given.
    #[test]
    fn every_ambient_source_feeds_the_guard_and_the_build_alike() {
        let selecting = [
            AmbientRustflags::Encoded("--cfg=x\u{1f}-Cpanic=immediate-abort".into()),
            AmbientRustflags::Plain("--cfg=x -Cpanic=immediate-abort".into()),
            AmbientRustflags::FromConfig(strings(&["--cfg=x", "-C", "panic=immediate-abort"])),
        ];
        for ambient in &selecting {
            assert!(
                ambient_immediate_abort_needs_build_std(StdMode::Precompiled, Some(ambient)),
                "{ambient:?}"
            );
            let (name, value) = add_required_rustflags(Some(ambient), false);
            let expected = if matches!(ambient, AmbientRustflags::Plain(_)) {
                "RUSTFLAGS"
            } else {
                "CARGO_ENCODED_RUSTFLAGS"
            };
            assert_eq!(name, expected, "{ambient:?}");
            assert!(value.contains("immediate-abort"), "{ambient:?}: {value}");
        }
    }

    #[test]
    fn config_rustflags_only_count_when_non_empty() {
        assert_eq!(AmbientRustflags::from_config(Vec::new()), None);
        assert_eq!(
            AmbientRustflags::from_config(strings(&["--cfg=x"])),
            Some(AmbientRustflags::FromConfig(strings(&["--cfg=x"])))
        );
    }

    #[test]
    fn encoded_rustflags_outrank_plain_ones_as_in_cargo() -> Result<()> {
        let os = |text: &str| Some(OsString::from(text));
        assert_eq!(
            AmbientRustflags::from_env(os("a"), os("b"))?,
            Some(AmbientRustflags::Encoded("a".into()))
        );
        assert_eq!(
            AmbientRustflags::from_env(None, os("b"))?,
            Some(AmbientRustflags::Plain("b".into()))
        );
        // Set-but-empty still shadows config, so it is not "nothing ambient".
        assert_eq!(
            AmbientRustflags::from_env(None, os(""))?,
            Some(AmbientRustflags::Plain(String::new()))
        );
        assert_eq!(AmbientRustflags::from_env(None, None)?, None);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_rustflags_are_an_error_not_a_lossy_guess() {
        use std::os::unix::ffi::OsStringExt;
        let invalid = || Some(OsString::from_vec(vec![b'-', b'C', 0xff, 0xfe]));
        for (encoded, plain, variable) in [
            (invalid(), None, "CARGO_ENCODED_RUSTFLAGS"),
            (None, invalid(), "RUSTFLAGS"),
        ] {
            let error = AmbientRustflags::from_env(encoded, plain)
                .expect_err("a non-UTF-8 value in effect must be refused");
            let message = error.to_string();
            assert!(message.contains(variable), "{message}");
            assert!(message.contains("not valid UTF-8"), "{message}");
        }
        // Cargo ignores `RUSTFLAGS` under the encoded variable, so a value it
        // would never read is no reason to refuse.
        assert_eq!(
            AmbientRustflags::from_env(Some("ok".into()), invalid()).ok(),
            Some(Some(AmbientRustflags::Encoded("ok".into())))
        );
    }

    #[test]
    fn immediate_abort_rustflags_are_added_only_when_asked_for() {
        // Nothing ambient: the function no longer reads the environment, so a
        // developer's own exported `-Cpanic=immediate-abort` cannot leak in.
        let (name, without) = add_required_rustflags(None, false);
        assert_eq!(name, "RUSTFLAGS");
        let (_, with) = add_required_rustflags(None, true);
        for flag in IMMEDIATE_ABORT_RUSTFLAGS {
            assert!(!without.contains(flag), "unasked-for `{flag}`");
            assert!(with.contains(flag), "missing `{flag}`");
        }
        for flag in REQUIRED_RUSTFLAGS {
            assert!(without.contains(flag), "missing `{flag}`");
            assert!(with.contains(flag), "missing `{flag}`");
        }
    }

    #[test]
    fn config_rustflags_accept_a_string_or_an_array() -> Result<()> {
        assert_eq!(
            parse_rustflags_value(r#""--cfg heap  -Cdebuginfo=1""#)?,
            ["--cfg", "heap", "-Cdebuginfo=1"]
        );
        // An array entry is one argument, spaces and all.
        assert_eq!(
            parse_rustflags_value(r#"["--cfg", "heap", "-Cllvm-args=a b"]"#)?,
            ["--cfg", "heap", "-Cllvm-args=a b"]
        );
        assert!(parse_rustflags_value("[]")?.is_empty());
        Ok(())
    }

    #[test]
    fn config_rustflags_reject_other_shapes() {
        assert!(parse_rustflags_value("3").is_err());
        assert!(parse_rustflags_value(r#"["--cfg", 3]"#).is_err());
        assert!(parse_rustflags_value("not json").is_err());
    }

    #[test]
    fn finds_only_cfg_tables_that_set_rustflags() -> Result<()> {
        let target = r#"{
            "cfg(all())": {"rustflags": ["--cfg", "x"]},
            "cfg(unix)": {"linker": "ld"},
            "x86_64-unknown-linux-gnu": {"rustflags": ["--cfg", "y"]}
        }"#;
        assert_eq!(cfg_rustflags_tables(target)?, ["cfg(all())"]);
        assert!(cfg_rustflags_tables("[]").is_err());
        Ok(())
    }

    #[test]
    fn non_empty_target_rustflags_shadow_build_rustflags_instead_of_merging() {
        assert_eq!(
            select_config_rustflags(
                Some(strings(&["--cfg", "build"])),
                Some(strings(&["--cfg", "target"])),
                &[]
            ),
            ConfigRustflags::Import(strings(&["--cfg", "target"]))
        );
        assert_eq!(
            select_config_rustflags(Some(strings(&["--cfg", "build"])), None, &[]),
            ConfigRustflags::Import(strings(&["--cfg", "build"]))
        );
        // An empty target list does not shadow `build`, as in Cargo.
        assert_eq!(
            select_config_rustflags(Some(strings(&["--cfg", "build"])), Some(Vec::new()), &[]),
            ConfigRustflags::Import(strings(&["--cfg", "build"]))
        );
        assert_eq!(
            select_config_rustflags(None, None, &[]),
            ConfigRustflags::Import(Vec::new())
        );
    }

    #[test]
    fn cfg_keyed_rustflags_are_refused_not_guessed() {
        let outcome = select_config_rustflags(
            Some(strings(&["--cfg", "build"])),
            None,
            &["cfg(all())".to_owned()],
        );
        let reason = match outcome {
            ConfigRustflags::Unsupported(reason) => reason,
            ConfigRustflags::Import(_) => String::new(),
        };
        assert!(reason.contains("cfg(all())"), "{reason}");
        assert!(reason.contains("RUSTFLAGS"), "{reason}");
    }

    #[test]
    fn imported_config_flags_come_first_and_the_panic_strategy_stays_last() {
        let imported = strings(&["--cfg", "heap", "-Cllvm-args=a b"]);
        let (name, merged) = add_required_rustflags(
            AmbientRustflags::from_config(imported.clone()).as_ref(),
            true,
        );
        assert_eq!(name, "CARGO_ENCODED_RUSTFLAGS");
        // With the user's own variable set, config is not consulted by Cargo,
        // so imported flags can never reach the build: `run` only imports
        // when `from_env` found nothing, and the variable's own value is what
        // the build is given.
        let user = AmbientRustflags::from_env(None, Some("--cfg user".into()))
            .ok()
            .flatten();
        let (shadowed_name, shadowed) = add_required_rustflags(user.as_ref(), false);
        assert_eq!(shadowed_name, "RUSTFLAGS");
        let parts: Vec<&str> = merged.split('\x1f').collect();
        assert_eq!(
            &parts[..3],
            ["--cfg", "heap", "-Cllvm-args=a b"],
            "{merged}"
        );
        assert!(parts.contains(&"--cfg=no_alloc_check"), "{merged}");
        assert_eq!(
            parts[parts.len() - IMMEDIATE_ABORT_RUSTFLAGS.len()..],
            *IMMEDIATE_ABORT_RUSTFLAGS,
            "{merged}"
        );
        assert!(!shadowed.contains("heap"), "{shadowed}");
        assert!(shadowed.starts_with("--cfg user"), "{shadowed}");
    }
}
