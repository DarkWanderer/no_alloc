//! What Cargo's own configuration says about the build, read through
//! `cargo config get` / `cargo metadata` so Cargo resolves precedence itself.

use crate::cargo_args::forward_options;
use anyhow::{bail, ensure, Context, Result};
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Config keys naming a program Cargo would run in front of (or instead of)
/// the driver. The driver must be the only thing between Cargo and rustc, or
/// a wrapper that short-circuits a unit (a compile cache on a hit) silently
/// drops that unit's roots from the report. Unlike the environment
/// variables, these are warned about and neutralised rather than refused:
/// the build blanks both wrapper variables (an empty value disables only its
/// own config key), so a configured wrapper (sccache in `~/.cargo/config.toml`)
/// cannot take effect and refusing would only block users for nothing.
///
/// `build.rustc` is deliberately not here: the shipped build would use the
/// configured compiler (a script adding `--cfg` flags, say) while the checked
/// build runs the toolchain `rustc`, so it is refused (`build_rustc_rejection`).
pub(crate) const WRAPPER_CONFIG_KEYS: &[&str] =
    &["build.rustc-wrapper", "build.rustc-workspace-wrapper"];

/// What `cargo config get` said about one key. `Unset` is a successful
/// answer; anything else Cargo did not understand is an error, never
/// `Unset`.
pub(crate) fn interpret_config_get(
    success: bool,
    stdout: &str,
    stderr: &str,
) -> Result<Option<String>> {
    if success {
        return Ok(Some(stdout.trim().to_owned()));
    }
    // Cargo has no distinct exit status for "key absent"; this message is the
    // only signal. Matching it narrowly keeps every other failure (a
    // malformed config file, say) fatal instead of reading as "unset" and
    // thereby as "nothing to import / reject".
    if stderr.contains("is not set") {
        return Ok(None);
    }
    bail!("`cargo config get` failed: {}", stderr.trim());
}

/// The `--config` overrides on the build's command line. They are global
/// Cargo options that feed the same configuration the build reads, so the
/// config queries must see them too. `--config` before `--` only: after it
/// the arguments belong to the test runner.
pub(crate) fn config_override_args(cargo_args: &[String]) -> Vec<String> {
    forward_options(cargo_args, &[], &["--config"])
}

pub(crate) fn cargo_config_get(
    cargo: &OsString,
    config_args: &[String],
    key: &str,
    extra: &[&str],
) -> Result<Option<String>> {
    let output = Command::new(cargo)
        .args(config_args)
        .args(["-Zunstable-options", "config", "get"])
        .args(extra)
        .arg(key)
        .output()
        .context("failed to spawn `cargo config get`")?;
    interpret_config_get(
        output.status.success(),
        &String::from_utf8_lossy(&output.stdout),
        &String::from_utf8_lossy(&output.stderr),
    )
    .with_context(|| format!("while reading Cargo config key `{key}`"))
}

/// `cargo config get --format json-value`: how every structured key is read.
pub(crate) fn cargo_config_json(
    cargo: &OsString,
    config_args: &[String],
    key: &str,
) -> Result<Option<String>> {
    cargo_config_get(cargo, config_args, key, &["--format", "json-value"])
}

/// A config value Cargo accepts either as one string or as an array of them.
pub(crate) enum StringOrArray {
    One(String),
    Many(Vec<String>),
}

/// The three failure messages for [`string_or_array`], each without the
/// offending value, which is appended after a `: `. Per-key text because the
/// messages name the key the user has to go and fix.
pub(crate) struct Wording {
    pub(crate) invalid_json: &'static str,
    pub(crate) non_string_entry: &'static str,
    pub(crate) neither: &'static str,
}

pub(crate) fn string_or_array(json: &str, wording: &Wording) -> Result<StringOrArray> {
    match serde_json::from_str::<serde_json::Value>(json).context(wording.invalid_json)? {
        serde_json::Value::String(value) => Ok(StringOrArray::One(value)),
        serde_json::Value::Array(items) => items
            .into_iter()
            .map(|item| match item {
                serde_json::Value::String(value) => Ok(value),
                other => bail!("{}: {other}", wording.non_string_entry),
            })
            .collect::<Result<_>>()
            .map(StringOrArray::Many),
        other => bail!("{}: {other}", wording.neither),
    }
}

/// Each item in backticks, comma-separated: how the refusal messages list the
/// offending targets, keys and tables.
pub(crate) fn backtick_list<T: std::fmt::Display>(items: impl IntoIterator<Item = T>) -> String {
    items
        .into_iter()
        .map(|item| format!("`{item}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Targets named by a `build.target` value: a string or an array of strings.
/// An empty string or array names nothing (Cargo treats it as unset).
pub(crate) fn parse_build_targets(json: &str) -> Result<Vec<String>> {
    let targets = match string_or_array(
        json,
        &Wording {
            invalid_json: "Cargo config `build.target` is not valid JSON",
            non_string_entry: "Cargo config `build.target` has a non-string entry",
            neither: "Cargo config `build.target` is neither a string nor an array",
        },
    )? {
        StringOrArray::One(target) => vec![target],
        StringOrArray::Many(targets) => targets,
    };
    Ok(targets
        .into_iter()
        .filter(|target| !target.trim().is_empty())
        .collect())
}

/// The refusal for a configured `build.target`. The checker always builds for
/// the host (`--target <host>`) and cannot honour another one, so the
/// config value would be dropped silently and `#[cfg(target_...)]` code the
/// shipped build compiles would never be seen.
pub(crate) fn build_target_rejection(targets: &[String]) -> String {
    format!(
        "Cargo config sets `build.target` to {} (via `[build] target`, `CARGO_BUILD_TARGET` or \
         `--config`), but the checker owns its target and always builds for the host, so the \
         shipped build's `cfg(target_...)` code would go unchecked; unset build.target / \
         CARGO_BUILD_TARGET",
        backtick_list(targets)
    )
}

/// The refusal for a configured `build.rustc`. The checked build compiles
/// with the toolchain's own `rustc` while `cargo build` would run the
/// configured one, which can differ in anything (extra `--cfg`/`-C` flags),
/// so verdicts could describe a different program than the one shipped.
pub(crate) fn build_rustc_rejection(value: &str) -> String {
    format!(
        "Cargo config sets `build.rustc` to {value} (via `[build] rustc` or `--config`), but \
         the checker compiles with the toolchain's own rustc, so the shipped build could differ \
         from the checked one (extra `--cfg` flags, say); unset build.rustc"
    )
}

/// Manifest keys that give a package its own build target. Cargo's
/// `cargo-features = ["per-package-target"]` honours them over the CLI-less
/// default, but the checker always passes `--target <host>`, which replaces
/// them, so the shipped build and the checked one would target different
/// platforms. `cargo metadata` does not expose either key on the pinned
/// Cargo, hence the manifest is read.
pub(crate) const PER_PACKAGE_TARGET_KEYS: [&str; 2] = ["default-target", "forced-target"];

/// Which of [`PER_PACKAGE_TARGET_KEYS`] a manifest's `[package]` sets.
pub(crate) fn per_package_target_keys(manifest: &str) -> Result<Vec<&'static str>> {
    let table: toml::Table = manifest.parse().context("manifest is not valid TOML")?;
    let package = table.get("package").and_then(toml::Value::as_table);
    Ok(PER_PACKAGE_TARGET_KEYS
        .into_iter()
        .filter(|key| package.is_some_and(|package| package.contains_key(*key)))
        .collect())
}

/// Manifest paths of the workspace members in `cargo metadata --no-deps` JSON.
pub(crate) fn workspace_manifest_paths(json: &str) -> Result<Vec<PathBuf>> {
    let metadata: serde_json::Value =
        serde_json::from_str(json).context("`cargo metadata` output is not valid JSON")?;
    let members: Vec<&str> = metadata["workspace_members"]
        .as_array()
        .context("`cargo metadata` output has no `workspace_members`")?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    let packages = metadata["packages"]
        .as_array()
        .context("`cargo metadata` output has no `packages`")?;
    packages
        .iter()
        .filter(|package| {
            package["id"]
                .as_str()
                .is_some_and(|id| members.contains(&id))
        })
        .map(|package| {
            package["manifest_path"]
                .as_str()
                .map(PathBuf::from)
                .context("a workspace package has no `manifest_path`")
        })
        .collect()
}

/// Argv (after the cargo binary) of the `cargo metadata` call that lists the
/// workspace the build will use. `--manifest-path` is an option of the
/// `metadata` subcommand, not a global one, so it must follow `metadata`;
/// `--config` is accepted globally and stays in front.
pub(crate) fn metadata_args(cargo_args: &[String]) -> Vec<String> {
    let mut argv = config_override_args(cargo_args);
    argv.extend(["metadata", "--format-version", "1", "--no-deps"].map(String::from));
    argv.extend(forward_options(cargo_args, &[], &["--manifest-path"]));
    argv
}

/// `(manifest path, keys)` for every workspace member that sets a per-package
/// target. Honours the build's `--manifest-path` and `--config` so it looks at
/// the workspace the build will.
pub(crate) fn workspace_per_package_targets(
    cargo: &OsString,
    cargo_args: &[String],
) -> Result<Vec<(String, Vec<&'static str>)>> {
    let output = Command::new(cargo)
        .args(metadata_args(cargo_args))
        .output()
        .context("failed to spawn `cargo metadata`")?;
    ensure!(
        output.status.success(),
        "`cargo metadata` failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let mut found = Vec::new();
    for path in workspace_manifest_paths(&String::from_utf8_lossy(&output.stdout))? {
        let manifest = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let keys = per_package_target_keys(&manifest)
            .with_context(|| format!("while reading {}", path.display()))?;
        if !keys.is_empty() {
            found.push((path.display().to_string(), keys));
        }
    }
    Ok(found)
}

pub(crate) fn per_package_target_rejection(found: &[(String, Vec<&str>)]) -> String {
    let described = found
        .iter()
        .map(|(manifest, keys)| format!("`{manifest}` ({})", backtick_list(keys)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "workspace manifest(s) {described} set a per-package target, but the checker owns its \
         target and always builds for the host, which replaces it, so the shipped build's target \
         code would go unchecked; remove `default-target` / `forced-target`"
    )
}

/// Keys of Cargo's `[env]` table that the checker reserves. Cargo fills any
/// unset variable of a spawned process from `[env]` (and overwrites set ones
/// for `force = true`), so an entry here can steer the driver — flip
/// `NO_ALLOC_WARN_ONLY`, or redirect `NO_ALLOC_FRAGMENT_DIR` so the report
/// sees no roots.
pub(crate) fn reserved_env_keys(env_json: &str) -> Result<Vec<String>> {
    let table: serde_json::Value =
        serde_json::from_str(env_json).context("Cargo `env` config is not valid JSON")?;
    let table = table
        .as_object()
        .context("Cargo `env` config is not a table")?;
    let mut keys: Vec<String> = table
        .keys()
        .filter(|key| key.starts_with("NO_ALLOC_"))
        .cloned()
        .collect();
    keys.sort();
    Ok(keys)
}

pub(crate) fn reserved_env_rejection(keys: &[String]) -> String {
    format!(
        "Cargo config `[env]` sets {}, which the checker reserves for steering its own driver \
         and would let the config change what is reported; remove the `NO_ALLOC_*` entries \
         from the `[env]` table",
        backtick_list(keys)
    )
}

/// `line` is `cargo config get --show-origin` output: `key = value # origin`,
/// where the origin is a file path or an environment variable. An unexpected
/// shape is quoted whole rather than dropped, so the message still says where
/// to look.
pub(crate) fn wrapper_warning(key: &str, line: &str) -> String {
    let described = line
        .strip_prefix(key)
        .and_then(|rest| rest.trim_start().strip_prefix('='))
        .and_then(|rest| rest.rsplit_once(" # "))
        .map(|(value, origin)| format!("set to {} from {origin}", value.trim()))
        .unwrap_or_else(|| format!("({line})"));
    format!(
        "ignoring `{key}` {described} for this run: the driver must be the only program wrapping rustc"
    )
}

/// Warnings for wrapper keys Cargo config (or its `CARGO_BUILD_*` environment
/// spelling) sets. `--show-origin` is what lets the message say where it came
/// from.
pub(crate) fn configured_wrappers(cargo: &OsString, config_args: &[String]) -> Result<Vec<String>> {
    let mut found = Vec::new();
    for key in WRAPPER_CONFIG_KEYS {
        if let Some(line) = cargo_config_get(cargo, config_args, key, &["--show-origin"])? {
            found.push(wrapper_warning(key, &line));
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::strings;

    #[test]
    fn metadata_args_put_manifest_path_after_the_subcommand() -> Result<()> {
        let to_args = |args: &[&str]| args.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>();
        for forwarded in [
            &["build", "--manifest-path", "/w/Cargo.toml"][..],
            &["build", "--manifest-path=/w/Cargo.toml"][..],
        ] {
            let mut cargo_args = to_args(forwarded);
            cargo_args.extend(to_args(&["--config", "a=1", "--", "--manifest-path", "x"]));
            let argv = metadata_args(&cargo_args);
            let metadata = argv
                .iter()
                .position(|a| a == "metadata")
                .context("metadata")?;
            let manifest = argv
                .iter()
                .position(|a| a.starts_with("--manifest-path"))
                .context("manifest-path")?;
            assert!(metadata < manifest, "{argv:?}");
            assert_eq!(&argv[..2], ["--config", "a=1"]);
            // Only the pre-`--` occurrence belongs to cargo.
            let count = argv
                .iter()
                .filter(|a| a.starts_with("--manifest-path"))
                .count();
            assert_eq!(count, 1, "{argv:?}");
        }
        Ok(())
    }

    #[test]
    fn build_target_values_parse_and_empty_ones_name_nothing() -> Result<()> {
        assert_eq!(
            parse_build_targets(r#""thumbv7em-none-eabihf""#)?,
            ["thumbv7em-none-eabihf"]
        );
        assert_eq!(parse_build_targets(r#"["a","b"]"#)?, ["a", "b"]);
        assert!(parse_build_targets(r#""""#)?.is_empty());
        assert!(parse_build_targets("[]")?.is_empty());
        assert!(parse_build_targets("3").is_err());
        let message = build_target_rejection(&["arm-none".to_owned()]);
        assert!(message.contains("`arm-none`"), "{message}");
        assert!(message.contains("CARGO_BUILD_TARGET"), "{message}");
        Ok(())
    }

    #[test]
    fn reserved_env_keys_finds_only_checker_variables() -> Result<()> {
        let json = r#"{"FOO":"1","NO_ALLOC_WARN_ONLY":"1","NO_ALLOC_FRAGMENT_DIR":{"value":"/x","force":true}}"#;
        assert_eq!(
            reserved_env_keys(json)?,
            ["NO_ALLOC_FRAGMENT_DIR", "NO_ALLOC_WARN_ONLY"]
        );
        assert!(reserved_env_keys(r#"{"FOO":"1"}"#)?.is_empty());
        assert!(reserved_env_keys("[]").is_err());
        let message = reserved_env_rejection(&["NO_ALLOC_WARN_ONLY".to_owned()]);
        assert!(message.contains("`NO_ALLOC_WARN_ONLY`"), "{message}");
        Ok(())
    }

    #[test]
    fn build_rustc_is_refused_not_warned_about() {
        let message = build_rustc_rejection("\"./my-rustc\"");
        assert!(message.contains("`build.rustc`"), "{message}");
        assert!(message.contains("\"./my-rustc\""), "{message}");
        assert!(message.contains("toolchain's own rustc"), "{message}");
        assert!(!WRAPPER_CONFIG_KEYS.contains(&"build.rustc"));
    }

    #[test]
    fn per_package_target_keys_are_found_in_the_package_table_only() -> Result<()> {
        let manifest = "[package]\nname = \"a\"\ndefault-target = \"x\"\nforced-target = \"y\"\n";
        assert_eq!(
            per_package_target_keys(manifest)?,
            ["default-target", "forced-target"]
        );
        assert!(per_package_target_keys("[package]\nname = \"a\"\n")?.is_empty());
        // A same-named key elsewhere is not the package setting.
        assert!(per_package_target_keys(
            "[package]\nname = \"a\"\n[package.metadata]\ndefault-target = 1\n"
        )?
        .is_empty());
        assert!(per_package_target_keys("[workspace]\nmembers = []\n")?.is_empty());
        assert!(per_package_target_keys("not toml [").is_err());
        Ok(())
    }

    #[test]
    fn workspace_manifest_paths_lists_only_members() -> Result<()> {
        let json = r#"{"workspace_members":["m 0.1.0 (path+file:///w/m)"],
            "packages":[{"id":"m 0.1.0 (path+file:///w/m)","manifest_path":"/w/m/Cargo.toml"},
                        {"id":"d 0.1.0 (path+file:///w/d)","manifest_path":"/w/d/Cargo.toml"}]}"#;
        assert_eq!(
            workspace_manifest_paths(json)?,
            [PathBuf::from("/w/m/Cargo.toml")]
        );
        assert!(workspace_manifest_paths("{}").is_err());
        Ok(())
    }

    #[test]
    fn per_package_target_rejection_names_the_manifest_and_key() {
        let message = per_package_target_rejection(&[(
            "/w/app/Cargo.toml".to_owned(),
            vec!["default-target"],
        )]);
        assert!(message.contains("`/w/app/Cargo.toml`"), "{message}");
        assert!(message.contains("`default-target`"), "{message}");
    }

    #[test]
    fn config_get_distinguishes_unset_from_failure() -> Result<()> {
        assert_eq!(
            interpret_config_get(true, "\"./skip.sh\"\n", "")?,
            Some("\"./skip.sh\"".to_owned())
        );
        assert_eq!(
            interpret_config_get(
                false,
                "",
                "error: config value `build.rustc-wrapper` is not set\n"
            )?,
            None
        );
        // A malformed config must not read as unset.
        let error = interpret_config_get(false, "", "error: could not parse TOML\n")
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
        assert!(error.contains("could not parse TOML"), "{error}");
        Ok(())
    }

    #[test]
    fn collects_config_overrides_before_the_runner_separator() {
        let args = strings(&[
            "--release",
            "--config",
            "build.rustflags=['--cfg=a']",
            "--config=build.rustc-wrapper='w'",
            "--",
            "--config",
            "ignored",
        ]);
        assert_eq!(
            config_override_args(&args),
            [
                "--config",
                "build.rustflags=['--cfg=a']",
                "--config=build.rustc-wrapper='w'"
            ]
        );
    }

    #[test]
    fn wrapper_warning_names_the_key_and_its_origin() {
        let from_file = wrapper_warning(
            "build.rustc-wrapper",
            "build.rustc-wrapper = \"./skip.sh\" # /p/.cargo/config.toml",
        );
        assert!(from_file.starts_with("ignoring `build.rustc-wrapper` set to"));
        assert!(from_file.contains("\"./skip.sh\""), "{from_file}");
        assert!(
            from_file.contains("from /p/.cargo/config.toml"),
            "{from_file}"
        );
        assert!(from_file.contains("for this run"), "{from_file}");
        let from_env = wrapper_warning(
            "build.rustc-workspace-wrapper",
            "build.rustc-workspace-wrapper = \"w\" # environment variable `CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER`",
        );
        assert!(
            from_env.contains("CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER"),
            "{from_env}"
        );
        // An output shape this code does not know is quoted, not dropped.
        let odd = wrapper_warning("build.rustc-wrapper", "something unexpected");
        assert!(odd.contains("something unexpected"), "{odd}");
    }
}
