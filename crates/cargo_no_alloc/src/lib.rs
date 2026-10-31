use anyhow::{bail, ensure, Context, Result};
use fs2::FileExt;
use no_alloc_report::{parse_root_spec, PanicStrategy, Report, ReportFragment, Verdict};
use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

// Shared with `build.rs` so the pin can't drift between the build-time
// check (the toolchain compiling this binary) and the run-time check below
// (the toolchain of the project being analyzed).
include!("toolchain_spec.rs");

// A cargo subcommand is expected to answer `--help`/`-h`; kept as a plain
// string (rather than built from `Invocation`'s fields) so it reads as the
// user-facing contract, not an implementation detail that happens to leak.
const HELP_TEXT: &str = "\
cargo no-alloc [OPTIONS] -- [build|test] [CARGO_ARGS...]

Statically checks that #[no_alloc]-marked function instances cannot reach
the global allocator, then runs the underlying Cargo command (`build` is
used when [build|test] is omitted; `check` is rejected because it never
produces the monomorphized instance graph the analysis runs on).

Options:
      --all-crates   Instrument every crate in the build (RUSTC_WRAPPER),
                     not just workspace members (RUSTC_WORKSPACE_WRAPPER)
      --build-std    Pass -Zbuild-std to Cargo
      --immediate-abort
                     Compile everything, std included, with
                     -Cpanic=immediate-abort so panic paths lower to a bare
                     abort() and can be checked instead of rejected.
                     Implies --build-std; cannot be combined with `test`.
                     See docs/iterators.md
      --warn-only    Report findings on stderr without failing the build
      --root PATH    Additionally check an unannotated function by its
                     canonical path (repeatable)
  -h, --help         Print this help and exit
  -V, --version      Print version information and exit";

// Printed unconditionally (not just under --warn-only) because a clean exit
// with zero roots checked is not a passing result, just an unverified one —
// see the README's "By default only workspace members are instrumented".
const ZERO_ROOTS_WARNING: &str = "no_alloc checked 0 root instances \
— nothing was analyzed. This usually means either no `#[no_alloc]` marker \
was reached while building, or the marker lives in a crate outside the \
workspace (pass --all-crates to instrument it).";

/// Added by `--immediate-abort`. `-Cpanic=immediate-abort` is still
/// unstable, hence the accompanying `-Zunstable-options`. Passing it here
/// rather than asking users to set `[profile.dev] panic = "immediate-abort"`
/// keeps the checked crate's own manifest stable-buildable: the manifest key
/// requires `cargo-features = ["panic-immediate-abort"]`, which makes the
/// whole manifest nightly-only (ADR 0002's zero-footprint rule).
const IMMEDIATE_ABORT_RUSTFLAGS: &[&str] = &["-Zunstable-options", "-Cpanic=immediate-abort"];

/// Cargo target selections that always pull in a libtest harness, which
/// cannot be built under `--immediate-abort` (see the `-- test` rejection).
///
/// Only these two: both include the unit-test targets of the lib and bins,
/// which use libtest whatever the manifest says. `--test`/`--bench` name one
/// target and `--benches` selects only bench targets — any of which may set
/// `harness = false` (as this repository's own benchmarks do) and then build
/// perfectly well under an abort strategy. Rejecting those would be a false
/// positive, and telling them apart needs per-target metadata this wrapper
/// does not read; the observed-strategy check after the build is what keeps
/// a mixed configuration from being *reported* either way.
const TEST_HARNESS_SELECTORS: &[&str] = &["--tests", "--all-targets"];

const REQUIRED_RUSTFLAGS: &[&str] = &[
    "--cfg=no_alloc_check",
    "--check-cfg=cfg(no_alloc_check)",
    "-Zcrate-attr=feature(register_tool)",
    "-Zcrate-attr=register_tool(no_alloc_tool)",
    "-Zalways-encode-mir",
    "-Zcrate-attr=feature(rustc_attrs)",
    // `rustc_attrs` (for `#[rustc_no_mir_inline]` on markers) is an internal
    // feature; without this every checked crate warns about it.
    "-Zcrate-attr=allow(internal_features)",
];
const CACHE_DIR_TAG: &str = "Signature: 8a477f597d28d172789f06886806bc55\n\
# This file is a cache directory tag created by cargo.\n\
# For information about cache directory tags see https://bford.info/cachedir/\n";

#[derive(Debug, PartialEq, Eq)]
struct Invocation {
    build_std: bool,
    immediate_abort: bool,
    all_crates: bool,
    warn_only: bool,
    roots: Vec<String>,
    command: String,
    cargo_args: Vec<String>,
}

// `--help`/`--version` short-circuit before an `Invocation` can even be
// formed (e.g. `--root` with no path would otherwise reject them), so they
// need their own variants rather than boolean fields on `Invocation`.
enum ParsedArgs {
    Help,
    Version,
    Run(Invocation),
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<ParsedArgs> {
    let mut args: Vec<_> = args.into_iter().collect();
    if !args.is_empty() {
        args.remove(0);
    }
    // Cargo passes the external subcommand name as argv[1].
    if args.first().is_some_and(|arg| arg == "no-alloc") {
        args.remove(0);
    }

    let mut build_std = false;
    let mut immediate_abort = false;
    let mut all_crates = false;
    let mut warn_only = false;
    let mut roots = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--" => {
                index += 1;
                break;
            }
            "--build-std" => build_std = true,
            // Implies --build-std: the panic strategy has to match all the
            // way down, and the precompiled sysroot was not built with it.
            "--immediate-abort" => {
                immediate_abort = true;
                build_std = true;
            }
            "--all-crates" => all_crates = true,
            "--warn-only" => warn_only = true,
            // A cargo subcommand is expected to answer these regardless of
            // what else is on the line, so they exit before anything past
            // this point (including "-- <bad cargo args>") is even parsed.
            "--help" | "-h" => return Ok(ParsedArgs::Help),
            "--version" | "-V" => return Ok(ParsedArgs::Version),
            "--root" => {
                index += 1;
                let root = args
                    .get(index)
                    .context("--root requires a non-empty path")?;
                ensure!(!root.trim().is_empty(), "--root requires a non-empty path");
                roots.push(root.trim().to_owned());
            }
            value if value.starts_with("--root=") => {
                let root = value.trim_start_matches("--root=").trim();
                ensure!(!root.is_empty(), "--root requires a non-empty path");
                roots.push(root.to_owned());
            }
            value if value.starts_with('-') => bail!("unknown checker option `{value}`"),
            _ => break,
        }
        index += 1;
    }

    let mut cargo = args[index..].to_vec();
    let command = if cargo
        .first()
        .is_none_or(|argument| argument.starts_with('-'))
    {
        "build".to_owned()
    } else {
        cargo.remove(0)
    };
    ensure!(
        command != "check",
        "`cargo check` is unsupported because it does not produce a monomorphized instance graph"
    );
    ensure!(
        matches!(command.as_str(), "build" | "test"),
        "unsupported Cargo command `{command}`; expected `build` or `test`"
    );
    // Caught here rather than left to rustc, which fails the build several
    // minutes into a sysroot rebuild with `building tests with panic=abort
    // is not supported without -Zpanic_abort_tests`. Cargo's own
    // `-Zpanic-abort-tests` does not help: it keys off the profile's panic
    // setting, and this strategy arrives through RUSTFLAGS instead.
    ensure!(
        !(immediate_abort && command == "test"),
        "`--immediate-abort` cannot be combined with `-- test`: rustc refuses to build a \
         test harness under an abort panic strategy. Use `-- build`, which is the supported \
         mode for meaningful results anyway (see README, \"Guarantee and limitations\")"
    );
    let cargo_option_end = cargo
        .iter()
        .position(|arg| arg == "--")
        .unwrap_or(cargo.len());
    for arg in &cargo[..cargo_option_end] {
        ensure!(
            arg != "--target"
                && !arg.starts_with("--target=")
                && arg != "--target-dir"
                && !arg.starts_with("--target-dir="),
            "Cargo target options conflict with no_alloc's dedicated checker target"
        );
        // The same wall `-- test` hits, reached through target selection
        // instead of the subcommand: these all compile libtest harnesses,
        // which rustc will not build under an abort panic strategy.
        let flag = arg.split('=').next().unwrap_or(arg);
        ensure!(
            !immediate_abort || !TEST_HARNESS_SELECTORS.contains(&flag),
            "`--immediate-abort` cannot build test-harness targets (`{flag}`): rustc refuses \
             to build a test harness under an abort panic strategy. Drop the target selection, \
             or check those targets without `--immediate-abort`"
        );
    }
    roots.sort();
    roots.dedup();
    Ok(ParsedArgs::Run(Invocation {
        build_std,
        immediate_abort,
        all_crates,
        warn_only,
        roots,
        command,
        cargo_args: cargo,
    }))
}

fn host_triple(rustc: &str) -> Result<String> {
    let output = Command::new(rustc)
        .args(["--print", "host-tuple"])
        .output()
        .with_context(|| format!("failed to run `{rustc} --print host-tuple`"))?;
    ensure!(
        output.status.success(),
        "`{rustc} --print host-tuple` exited with {}",
        output.status
    );
    Ok(String::from_utf8(output.stdout)
        .context("host-tuple output is not UTF-8")?
        .trim()
        .to_owned())
}

/// Variables that make Cargo compile with a program other than the
/// toolchain's own `rustc`. The driver links its own rustc and ignores the
/// binary Cargo hands it, so a script behind one of these (adding `--cfg`
/// flags, say) shapes the shipped build but never the checked one.
const RUSTC_OVERRIDE_VARS: &[&str] = &["RUSTC", "CARGO_BUILD_RUSTC"];

/// The override variables `is_set` reports, in declaration order. Taking the
/// predicate keeps this testable without touching process-wide state.
fn rustc_overrides_set(is_set: impl Fn(&str) -> bool) -> Vec<&'static str> {
    RUSTC_OVERRIDE_VARS
        .iter()
        .copied()
        .filter(|name| is_set(name))
        .collect()
}

fn reject_rustc_override() -> Result<()> {
    let set = rustc_overrides_set(|name| env::var_os(name).is_some());
    ensure!(
        set.is_empty(),
        "{} is set, but the checker compiles with the toolchain's own rustc and cannot honour a \
         substitute: a custom rustc would shape the shipped build and not the checked one. \
         Unset it; the checker owns its compiler",
        set.join("/")
    );
    Ok(())
}

/// The `rustc` Cargo would find on `PATH`, as an absolute path. Not
/// canonicalized: under rustup this is a symlink to the `rustup` proxy, which
/// only dispatches on the name it was invoked by.
///
/// The probes and the build both use this one path (the build is handed it as
/// `RUSTC`), so what was validated is what runs.
fn resolve_rustc() -> Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let path = env::var_os("PATH").context("PATH is not set, so `rustc` cannot be found")?;
    for directory in env::split_paths(&path) {
        let candidate = directory.join("rustc");
        let executable = fs::metadata(&candidate)
            .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0);
        if executable {
            return std::path::absolute(&candidate)
                .with_context(|| format!("failed to absolutize {}", candidate.display()));
        }
    }
    bail!("no executable `rustc` found on PATH")
}

// Checks the toolchain of the project being analyzed, mirroring build.rs's
// compile-time check on the toolchain that built this binary. Without this,
// a project pinned to a different toolchain (e.g. via rust-toolchain.toml)
// resolves the wrong sysroot in `sysroot_lib_dir` below, which otherwise
// only ever surfaces many steps later as a raw dynamic-linker error.
// `rustc` is threaded in explicitly (rather than read from the environment
// here) so this predicate can be unit tested against a fake rustc without
// touching process-wide env state.
fn verify_pinned_toolchain(rustc: &str) -> Result<()> {
    let output = Command::new(rustc)
        .arg("-vV")
        .output()
        .with_context(|| format!("failed to run `{rustc} -vV`"))?;
    ensure!(
        output.status.success(),
        "`{rustc} -vV` exited with {}",
        output.status
    );
    let version = String::from_utf8(output.stdout).context("rustc version is not UTF-8")?;
    let host = version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .context("rustc did not report a host triple")?;
    ensure!(
        is_pinned_toolchain(&version, host),
        toolchain_mismatch_message(&version)
    );
    Ok(())
}

fn sysroot_lib_dir(rustc: &str) -> Result<PathBuf> {
    let output = Command::new(rustc)
        .args(["--print", "sysroot"])
        .output()
        .with_context(|| format!("failed to run `{rustc} --print sysroot`"))?;
    ensure!(
        output.status.success(),
        "`{rustc} --print sysroot` exited with {}",
        output.status
    );
    let sysroot = String::from_utf8(output.stdout)
        .context("sysroot path is not valid UTF-8")?
        .trim()
        .to_owned();
    Ok(PathBuf::from(sysroot).join("lib"))
}

// `no-alloc-driver` dynamically links `librustc_driver-*.so`/`libLLVM.so`,
// which live in the sysroot rather than next to the binary. A binary built
// on one machine (e.g. CI) and run on another can't rely on a path baked in
// at build time, so this is resolved fresh from whatever `rustc` is active
// for the invoking project and threaded through as `LD_LIBRARY_PATH`.
fn combined_ld_library_path(sysroot_lib: &Path, existing: Option<&std::ffi::OsStr>) -> OsString {
    match existing {
        Some(existing) if !existing.is_empty() => {
            let mut combined = sysroot_lib.as_os_str().to_owned();
            combined.push(":");
            combined.push(existing);
            combined
        }
        _ => sysroot_lib.as_os_str().to_owned(),
    }
}

fn find_driver() -> Result<PathBuf> {
    let exe = env::current_exe().context("failed to resolve current executable")?;
    let sibling = exe
        .parent()
        .context("current executable has no parent")?
        .join("no-alloc-driver");
    Ok(if sibling.is_file() {
        sibling
    } else {
        PathBuf::from("no-alloc-driver")
    })
}

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
fn last_panic_setting<'a>(tokens: &[&'a str]) -> Option<&'a str> {
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
fn declares_immediate_abort(encoded: Option<&str>, plain: Option<&str>) -> bool {
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

/// Whether the build the driver just ran mixes an immediate-abort crate
/// with a standard library that was not rebuilt to match.
///
/// What makes an observed `ImmediateAbort` strategy safe is that the
/// sysroot was rebuilt *in this build* — which is what `--build-std` does:
/// Cargo applies the same profile and the same `RUSTFLAGS` to those units
/// too. So `--build-std` with the strategy set by hand (the manifest, a
/// `config.toml`, `--profile`) is coherent and not flagged; only an
/// immediate-abort crate built against the untouched precompiled sysroot
/// is a mix worth rejecting.
///
/// `any_immediate_abort` comes from scanning the raw fragments
/// (`aggregate`), not from `report.panic_strategy`: the merged field goes to
/// `None` on any disagreement between fragments, which would silently hide
/// exactly the mix this check exists to catch (a target-specific config
/// selecting the strategy for the checked crate while a wrapped host unit
/// compiles under something else).
fn mixed_sysroot_panic_strategy(build_std: bool, any_immediate_abort: bool) -> bool {
    !build_std && any_immediate_abort
}

/// Marks a report so that `Report::is_success()` says `false` on it, the
/// instant a mixed sysroot is detected — before the report is written to
/// disk. Without this, `report.json` could show every root passing (a
/// trivial function has no reason to care what std's panic strategy is)
/// while the process that produced it exits nonzero: a tool reading the
/// file directly, rather than this process's own exit code, would see a
/// false success. `selection_errors` is the existing channel for "this
/// report should not be trusted as-is" (already used for an unmatched
/// `--root`), so this reuses it rather than adding a second one.
fn taint_report_for_mixed_sysroot(report: &mut Report, mixed_sysroot: bool) {
    if mixed_sysroot {
        report.selection_errors.push(
            "this build compiled under `-Cpanic=immediate-abort` without rebuilding the \
             standard library (`--build-std`); every verdict above was produced against a \
             sysroot whose panic runtime does not match the strategy claimed"
                .to_owned(),
        );
    }
}

/// Whether the pre-flight ambient-flag guard should reject the run.
///
/// The escape hatch is `--build-std`, not `--immediate-abort` specifically:
/// `--immediate-abort` always implies `--build-std` (`parse_args` sets both
/// together), but the reverse isn't required, and a hand-set ambient
/// strategy plus plain `--build-std` rebuilds the sysroot under the same
/// inherited flags — exactly the case `mixed_sysroot_panic_strategy` above
/// already treats as coherent. Gating this earlier guard on
/// `inv.immediate_abort` instead rejected that same legitimate combination
/// before the build ever ran.
fn ambient_immediate_abort_needs_build_std(
    build_std: bool,
    encoded: Option<&str>,
    plain: Option<&str>,
) -> bool {
    !build_std && declares_immediate_abort(encoded, plain)
}

/// Config keys naming a program Cargo would run in front of (or instead of)
/// the driver. The driver must be the only thing between Cargo and rustc, or
/// a wrapper that short-circuits a unit (a compile cache on a hit) silently
/// drops that unit's roots from the report. Unlike the environment
/// variables, these are warned about and neutralised rather than refused:
/// the build blanks both wrapper variables (an empty value disables only its
/// own config key), so a configured wrapper (sccache in `~/.cargo/config.toml`)
/// cannot take effect and refusing would only block users for nothing.
///
/// `build.rustc` is in the sweep for the same reason: the build is handed the
/// validated toolchain `rustc` through `RUSTC`, which outranks the config key,
/// so a configured substitute cannot take effect either.
const WRAPPER_CONFIG_KEYS: &[&str] = &[
    "build.rustc-wrapper",
    "build.rustc-workspace-wrapper",
    "build.rustc",
];

/// What `cargo config get` said about one key. `Unset` is a successful
/// answer; anything else Cargo did not understand is an error, never
/// `Unset`.
fn interpret_config_get(success: bool, stdout: &str, stderr: &str) -> Result<Option<String>> {
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
fn config_override_args(cargo_args: &[String]) -> Vec<String> {
    let mut overrides = Vec::new();
    let mut args = cargo_args.iter().take_while(|arg| *arg != "--");
    while let Some(arg) = args.next() {
        if arg == "--config" {
            overrides.push(arg.clone());
            overrides.extend(args.next().cloned());
        } else if arg.starts_with("--config=") {
            overrides.push(arg.clone());
        }
    }
    overrides
}

fn cargo_config_get(
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

/// Cargo accepts rustflags as one whitespace-split string or as an array.
fn parse_rustflags_value(json: &str) -> Result<Vec<String>> {
    match serde_json::from_str::<serde_json::Value>(json)
        .context("Cargo config rustflags are not valid JSON")?
    {
        serde_json::Value::String(flags) => {
            Ok(flags.split_whitespace().map(str::to_owned).collect())
        }
        serde_json::Value::Array(items) => items
            .into_iter()
            .map(|item| match item {
                serde_json::Value::String(flag) => Ok(flag),
                other => bail!("Cargo config rustflags contain a non-string entry: {other}"),
            })
            .collect(),
        other => bail!("Cargo config rustflags are neither a string nor an array: {other}"),
    }
}

/// Names of `target.'cfg(...)'` tables that set `rustflags`. Evaluating a
/// cfg expression is Cargo's job against its own target description; this
/// wrapper only needs to know whether such a table exists.
fn cfg_rustflags_tables(target_json: &str) -> Result<Vec<String>> {
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
enum ConfigRustflags {
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
fn select_config_rustflags(
    build: Option<Vec<String>>,
    host_target: Option<Vec<String>>,
    cfg_tables: &[String],
) -> ConfigRustflags {
    if !cfg_tables.is_empty() {
        return ConfigRustflags::Unsupported(format!(
            "Cargo config sets rustflags under {} which this checker cannot evaluate, so it \
             cannot tell which flags the shipped build uses; config rustflags keyed on \
             `cfg(...)` are not supported — put the flags in `RUSTFLAGS` instead",
            cfg_tables
                .iter()
                .map(|name| format!("`target.'{name}'`"))
                .collect::<Vec<_>>()
                .join(", ")
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
fn config_rustflags(
    cargo: &OsString,
    config_args: &[String],
    host: &str,
) -> Result<ConfigRustflags> {
    let get = |key: &str| -> Result<Option<Vec<String>>> {
        cargo_config_get(cargo, config_args, key, &["--format", "json-value"])?
            .map(|json| parse_rustflags_value(&json))
            .transpose()
            .with_context(|| format!("while reading Cargo config key `{key}`"))
    };
    let cfg_tables =
        match cargo_config_get(cargo, config_args, "target", &["--format", "json-value"])? {
            Some(json) => cfg_rustflags_tables(&json)?,
            None => Vec::new(),
        };
    Ok(select_config_rustflags(
        get("build.rustflags")?,
        get(&format!("target.{host}.rustflags"))?,
        &cfg_tables,
    ))
}

/// Targets named by a `build.target` value: a string or an array of strings.
/// An empty string or array names nothing (Cargo treats it as unset).
fn parse_build_targets(json: &str) -> Result<Vec<String>> {
    match serde_json::from_str::<serde_json::Value>(json)
        .context("Cargo config `build.target` is not valid JSON")?
    {
        serde_json::Value::String(target) => Ok(Some(target)
            .filter(|target| !target.trim().is_empty())
            .into_iter()
            .collect()),
        serde_json::Value::Array(items) => items
            .into_iter()
            .map(|item| match item {
                serde_json::Value::String(target) => Ok(target),
                other => bail!("Cargo config `build.target` has a non-string entry: {other}"),
            })
            .filter(|target| target.as_ref().map_or(true, |t| !t.trim().is_empty()))
            .collect(),
        other => bail!("Cargo config `build.target` is neither a string nor an array: {other}"),
    }
}

/// The refusal for a configured `build.target`. The checker always builds for
/// the host (`--target <host>`) and cannot honour another one, so the
/// config value would be dropped silently and `#[cfg(target_...)]` code the
/// shipped build compiles would never be seen.
fn build_target_rejection(targets: &[String]) -> String {
    format!(
        "Cargo config sets `build.target` to {} (via `[build] target`, `CARGO_BUILD_TARGET` or \
         `--config`), but the checker owns its target and always builds for the host, so the \
         shipped build's `cfg(target_...)` code would go unchecked; unset build.target / \
         CARGO_BUILD_TARGET",
        targets
            .iter()
            .map(|target| format!("`{target}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Keys of Cargo's `[env]` table that the checker reserves. Cargo fills any
/// unset variable of a spawned process from `[env]` (and overwrites set ones
/// for `force = true`), so an entry here can steer the driver — flip
/// `NO_ALLOC_WARN_ONLY`, or redirect `NO_ALLOC_FRAGMENT_DIR` so the report
/// sees no roots.
fn reserved_env_keys(env_json: &str) -> Result<Vec<String>> {
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

fn reserved_env_rejection(keys: &[String]) -> String {
    format!(
        "Cargo config `[env]` sets {}, which the checker reserves for steering its own driver \
         and would let the config change what is reported; remove the `NO_ALLOC_*` entries \
         from the `[env]` table",
        keys.iter()
            .map(|key| format!("`{key}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// A report that records why the run was refused, so `is_success()` is
/// false for anyone reading the persisted file.
fn rejected_report(mut reasons: Vec<String>) -> Report {
    reasons.sort();
    reasons.dedup();
    Report {
        selection_errors: reasons,
        ..Report::default()
    }
}

/// `line` is `cargo config get --show-origin` output: `key = value # origin`,
/// where the origin is a file path or an environment variable. An unexpected
/// shape is quoted whole rather than dropped, so the message still says where
/// to look.
fn wrapper_warning(key: &str, line: &str) -> String {
    let described = line
        .strip_prefix(key)
        .and_then(|rest| rest.trim_start().strip_prefix('='))
        .and_then(|rest| rest.rsplit_once(" # "))
        .map(|(value, origin)| format!("set to {} from {origin}", value.trim()))
        .unwrap_or_else(|| format!("({line})"));
    let reason = if key == "build.rustc" {
        "the checker compiles with the toolchain's own rustc"
    } else {
        "the driver must be the only program wrapping rustc"
    };
    format!("ignoring `{key}` {described} for this run: {reason}")
}

/// Warnings for wrapper keys Cargo config (or its `CARGO_BUILD_*` environment
/// spelling) sets. `--show-origin` is what lets the message say where it came
/// from.
fn configured_wrappers(cargo: &OsString, config_args: &[String]) -> Result<Vec<String>> {
    let mut found = Vec::new();
    for key in WRAPPER_CONFIG_KEYS {
        if let Some(line) = cargo_config_get(cargo, config_args, key, &["--show-origin"])? {
            found.push(wrapper_warning(key, &line));
        }
    }
    Ok(found)
}

fn add_required_rustflags(command: &mut Command, immediate_abort: bool, imported: &[String]) {
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
    // Config flags are carried in the encoded variable, which has no
    // word-splitting, so an array entry containing a space survives intact.
    // They are only a fallback: the environment, when set, is what Cargo
    // would have used and config would have been ignored.
    let imported_encoded = (!imported.is_empty() && env::var_os("RUSTFLAGS").is_none())
        .then(|| OsString::from(imported.join("\x1f")));
    if let Some(encoded) = env::var_os("CARGO_ENCODED_RUSTFLAGS").or(imported_encoded) {
        let mut value = encoded.to_string_lossy().into_owned();
        let existing: HashSet<String> = value.split('\x1f').map(str::to_owned).collect();
        let push = |flag: &str, value: &mut String| {
            if !value.is_empty() {
                value.push('\x1f');
            }
            value.push_str(flag);
        };
        for flag in REQUIRED_RUSTFLAGS {
            if !existing.contains(*flag) {
                push(flag, &mut value);
            }
        }
        for flag in panic_strategy {
            push(flag, &mut value);
        }
        command.env("CARGO_ENCODED_RUSTFLAGS", value);
    } else {
        let mut value = env::var("RUSTFLAGS").unwrap_or_default();
        let push = |flag: &str, value: &mut String| {
            if !value.is_empty() {
                value.push(' ');
            }
            value.push_str(flag);
        };
        for flag in REQUIRED_RUSTFLAGS {
            if !value.split_whitespace().any(|existing| existing == *flag) {
                push(flag, &mut value);
            }
        }
        for flag in panic_strategy {
            push(flag, &mut value);
        }
        command.env("RUSTFLAGS", value);
    }
}

fn cargo_status(mut command: Command) -> Result<ExitStatus> {
    command.status().context("failed to spawn Cargo")
}

fn clear_checker_artifacts(
    cargo: &OsString,
    inv: &Invocation,
    target_dir: &Path,
    host: &str,
    rustc: &Path,
) -> Result<()> {
    let mut clean = Command::new(cargo);
    clean.arg("clean");
    // `cargo clean --target` probes the compiler too, so a `build.rustc` from
    // config would otherwise run here, before the build's own override.
    clean.env("RUSTC", rustc);
    if !inv.all_crates {
        clean.arg("--workspace");
    }
    clean
        .args(clean_selection_args(&inv.cargo_args))
        .arg("--target")
        .arg(host)
        .arg("--target-dir")
        .arg(target_dir);
    let status = cargo_status(clean)?;
    ensure!(status.success(), "Cargo clean exited with {status}");
    Ok(())
}

/// The build's own options that decide *which* artifacts `cargo clean`
/// must remove. Without them `cargo clean --workspace` clears only the dev
/// profile of the workspace in the current directory, so a `--release` or
/// `--profile` build (or one of another `--manifest-path`) found its
/// workspace units fresh, the driver never ran on them, and their roots
/// silently dropped out of a report that still read as a success.
fn clean_selection_args(cargo_args: &[String]) -> Vec<String> {
    const WITH_VALUE: [&str; 3] = ["--profile", "--manifest-path", "--config"];
    let mut selected = Vec::new();
    let mut args = cargo_args.iter().take_while(|arg| *arg != "--");
    while let Some(arg) = args.next() {
        if arg == "--release" || arg == "-r" {
            selected.push(arg.clone());
        } else if WITH_VALUE.contains(&arg.as_str()) {
            selected.push(arg.clone());
            selected.extend(args.next().cloned());
        } else if WITH_VALUE.iter().any(|flag| {
            arg.strip_prefix(flag)
                .is_some_and(|rest| rest.starts_with('='))
        }) {
            selected.push(arg.clone());
        }
    }
    selected
}

/// A Cargo failure can stop the build before every wrapped unit was
/// compiled, so the fragments on disk may cover only part of it. When
/// nothing else already marks the report as failing, say so in the
/// persisted file — not only in this process's exit code — so a consumer
/// reading `report.json` cannot take a partial run for a passing one.
fn taint_report_for_failed_build(report: &mut Report, cargo_succeeded: bool) {
    if !cargo_succeeded && report.is_success() {
        report.selection_errors.push(
            "Cargo did not complete the build, so some roots may never have been checked"
                .to_owned(),
        );
    }
}

/// `Report::merge`'s agreed-strategy answer is the right thing for
/// `report.json` to state (ADR 0006) — it is silent rather than guessing
/// when fragments disagree. It is the wrong question for this wrapper's own
/// safety net, though: a target-specific `.cargo/config.toml` can compile
/// the checked crate under `ImmediateAbort` while a wrapped host unit (a
/// build script with a checked root of its own) compiles under `Unwind`,
/// and the disagreement collapses the merged strategy to `None` — silently
/// clearing the one signal `mixed_sysroot_panic_strategy` needs to catch
/// exactly that case. So this is answered directly from the fragments,
/// before merging can lose it: "did *any* verdict-bearing fragment observe
/// `ImmediateAbort`", regardless of what the others said.
fn aggregate(fragment_dir: &Path, requested: &[String]) -> Result<(Report, bool)> {
    let mut reports = Vec::new();
    let mut matched = HashSet::new();
    if fragment_dir.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(fragment_dir)?.collect::<std::io::Result<_>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let fragment: ReportFragment = serde_json::from_reader(File::open(entry.path())?)
                .with_context(|| format!("failed to parse {}", entry.path().display()))?;
            matched.extend(fragment.matched_root_specs);
            reports.push(fragment.report);
        }
    }
    let any_immediate_abort = reports.iter().any(|report| {
        report.checked_an_instance() && report.panic_strategy == Some(PanicStrategy::ImmediateAbort)
    });
    let mut report = Report::merge(reports);
    for root in requested {
        if !matched.contains(root) {
            report.selection_errors.push(format!(
                "requested root `{root}` did not match any item in the build"
            ));
        }
    }
    report.selection_errors.sort();
    report.selection_errors.dedup();
    Ok((report, any_immediate_abort))
}

pub fn run(args: impl IntoIterator<Item = String>) -> Result<()> {
    let mut inv = match parse_args(args)? {
        // Neither prints anything about the pinned toolchain or spawns a
        // build, so both work even when the active toolchain is wrong.
        ParsedArgs::Help => {
            println!("{HELP_TEXT}");
            return Ok(());
        }
        ParsedArgs::Version => {
            println!("cargo-no-alloc {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        ParsedArgs::Run(inv) => inv,
    };
    let cwd = env::current_dir().context("failed to determine current directory")?;
    let target_dir = cwd.join("target/no-alloc");
    let fragment_dir = target_dir.join("fragments");
    fs::create_dir_all(cwd.join("target"))?;
    let lock = File::create(cwd.join("target/no-alloc.lock"))?;
    lock.lock_exclusive()
        .context("failed to lock target/no-alloc.lock")?;
    // Every exit below this point that bails before the final write would
    // otherwise leave the previous run's report in place, readable as this
    // run's result.
    let report_path = target_dir.join("report.json");
    match fs::remove_file(&report_path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(error).context("failed to remove the previous report.json");
        }
        _ => {}
    }
    // Checked before anything else touches Cargo: a project pinned to the
    // wrong toolchain otherwise fails much further down, in
    // `sysroot_lib_dir`, as a raw dynamic-linker error instead of this
    // actionable message.
    //
    // The override check comes first so that no probe ever runs a substitute
    // compiler, and the probes below use the same resolved path the build is
    // handed.
    reject_rustc_override()?;
    let rustc_path = resolve_rustc()?;
    let rustc = rustc_path
        .to_str()
        .context("the path of `rustc` is not valid UTF-8")?
        .to_owned();
    verify_pinned_toolchain(&rustc)?;
    if env::var("NO_ALLOC_WARN_ONLY").as_deref() == Ok("1") {
        inv.warn_only = true;
    }
    if env::var_os("RUSTC_WRAPPER").is_some() || env::var_os("RUSTC_WORKSPACE_WRAPPER").is_some() {
        bail!(
            "existing RUSTC_WRAPPER/RUSTC_WORKSPACE_WRAPPER configuration is incompatible with cargo-no-alloc"
        );
    }
    if let Ok(existing) = env::var("NO_ALLOC_ROOTS") {
        inv.roots.extend(parse_root_spec(&existing));
        inv.roots.sort();
        inv.roots.dedup();
    }

    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let host = host_triple(&rustc)?;
    // The fragment directory exists before Cargo first sees this custom
    // target, so create Cargo's safety tag before asking `cargo clean` to
    // manage it on this and later runs.
    fs::create_dir_all(&target_dir)?;
    fs::write(target_dir.join("CACHEDIR.TAG"), CACHE_DIR_TAG)?;

    // Configuration the environment-variable checks above cannot see. A
    // run that cannot be made faithful is recorded in a persisted report
    // (not only refused on stderr), the same way a mixed sysroot is, so a
    // consumer reading `report.json` never finds a clean file for a build
    // that was not checked.
    let config_args = config_override_args(&inv.cargo_args);
    // Neutralised below by blanking the wrapper variables, so only a warning.
    for warning in configured_wrappers(&cargo, &config_args)? {
        eprintln!("warning: {warning}");
    }
    let mut rejections = Vec::new();
    if let Some(json) = cargo_config_get(
        &cargo,
        &config_args,
        "build.target",
        &["--format", "json-value"],
    )? {
        let targets = parse_build_targets(&json)?;
        if !targets.is_empty() {
            rejections.push(build_target_rejection(&targets));
        }
    }
    if let Some(json) = cargo_config_get(&cargo, &config_args, "env", &["--format", "json-value"])?
    {
        let keys = reserved_env_keys(&json)?;
        if !keys.is_empty() {
            rejections.push(reserved_env_rejection(&keys));
        }
    }
    let mut imported_rustflags = Vec::new();
    // Cargo ignores config rustflags entirely once either variable is set
    // (even to an empty value), so the caller's environment is already what
    // plain `cargo build` would use and there is nothing to import.
    let env_rustflags =
        env::var("CARGO_ENCODED_RUSTFLAGS").is_ok() || env::var("RUSTFLAGS").is_ok();
    if !env_rustflags {
        match config_rustflags(&cargo, &config_args, &host)? {
            ConfigRustflags::Import(flags) => {
                if !flags.is_empty() {
                    tracing::info!(?flags, "importing rustflags from Cargo config");
                }
                imported_rustflags = flags;
            }
            ConfigRustflags::Unsupported(reason) => rejections.push(reason),
        }
    }
    if !rejections.is_empty() {
        let report = rejected_report(rejections);
        report
            .write_to_file(&report_path)
            .context("failed to write final report.json")?;
        for error in &report.selection_errors {
            eprintln!("error: {error}");
        }
        bail!("this build's configuration cannot be checked faithfully; nothing was built");
    }

    // The strategy can arrive through the caller's own flags instead of the
    // checker option, and then nothing rebuilds the sysroot: the crate
    // compiles under immediate-abort while std keeps the panic runtime it
    // was precompiled with, and the report would claim `immediate_abort` for
    // a build where std's panic paths are not compiled that way at all.
    //
    // The escape hatch is `--build-std`, not `--immediate-abort` —
    // `--immediate-abort` always implies `--build-std` (see `parse_args`),
    // but the reverse isn't required: a hand-set ambient strategy plus plain
    // `--build-std` rebuilds the sysroot under the same inherited flags and
    // is exactly as coherent as the flag, which is what the post-build
    // check's own `mixed_sysroot_panic_strategy` already agrees is fine.
    // Gating on `inv.immediate_abort` here instead rejected that legitimate
    // combination before ever reaching the build.
    // Imported config flags stand in for the encoded variable here: they are
    // what rustc would see, and `-Cpanic=immediate-abort` in config is as
    // much an ambient selection as one in the environment.
    let ambient_encoded = env::var("CARGO_ENCODED_RUSTFLAGS")
        .ok()
        .or_else(|| (!imported_rustflags.is_empty()).then(|| imported_rustflags.join("\x1f")));
    ensure!(
        !ambient_immediate_abort_needs_build_std(
            inv.build_std,
            ambient_encoded.as_deref(),
            env::var("RUSTFLAGS").ok().as_deref(),
        ),
        "the environment's or Cargo config's Rust flags already select `-Cpanic=immediate-abort`, but neither \
         `--immediate-abort` nor `--build-std` was passed. That strategy only means what the \
         report says it means if the standard library is rebuilt with it too; pass \
         `--immediate-abort` (which does both) or `--build-std` (if you are setting the \
         strategy yourself)"
    );

    clear_checker_artifacts(&cargo, &inv, &target_dir, &host, &rustc_path)?;
    if fragment_dir.exists() {
        fs::remove_dir_all(&fragment_dir)?;
    }
    fs::create_dir_all(&fragment_dir)?;

    let mut command = Command::new(&cargo);
    command.arg(&inv.command);
    let split = inv
        .cargo_args
        .iter()
        .position(|arg| arg == "--")
        .unwrap_or(inv.cargo_args.len());
    command.args(&inv.cargo_args[..split]);
    command
        .arg("--target")
        .arg(&host)
        .arg("--target-dir")
        .arg(&target_dir);
    if inv.build_std {
        command.arg("-Zbuild-std");
    }
    command.args(&inv.cargo_args[split..]);
    add_required_rustflags(&mut command, inv.immediate_abort, &imported_rustflags);
    command.env("NO_ALLOC_FRAGMENT_DIR", &fragment_dir);
    command.env("NO_ALLOC_ROOTS", inv.roots.join(","));
    // Read back by the driver into each fragment's `Environment` (F4): these
    // are invocation-level flags, not something `Session` exposes, so they
    // have to cross the process boundary as plain env vars like everything
    // else the driver needs from `cargo-no-alloc` itself.
    command.env(
        "NO_ALLOC_ALL_CRATES",
        if inv.all_crates { "1" } else { "0" },
    );
    command.env("NO_ALLOC_BUILD_STD", if inv.build_std { "1" } else { "0" });
    // Always set, never removed: a removed variable is a hole Cargo's `[env]`
    // table would fill. The driver treats only exactly "1" as warn-only.
    command.env("NO_ALLOC_WARN_ONLY", if inv.warn_only { "1" } else { "0" });
    // Hand Cargo the rustc that was validated above (`RUSTC` outranks
    // `build.rustc`), and tell the driver which one that is so it can refuse
    // to compile when anything else arrives as its argv[1].
    command.env("RUSTC", &rustc_path);
    command.env("NO_ALLOC_EXPECTED_RUSTC", &rustc_path);
    let driver = find_driver()?;
    let sysroot_lib = sysroot_lib_dir(&rustc)?;
    command.env(
        "LD_LIBRARY_PATH",
        combined_ld_library_path(&sysroot_lib, env::var_os("LD_LIBRARY_PATH").as_deref()),
    );
    // The variable the checker is not using is blanked: an empty value
    // overrides the matching config key (verified on the pinned Cargo), so
    // a wrapper configured after the check above ran cannot slip in front of
    // the driver.
    if inv.all_crates {
        command.env("RUSTC_WRAPPER", driver);
        command.env("RUSTC_WORKSPACE_WRAPPER", "");
    } else {
        command.env("RUSTC_WORKSPACE_WRAPPER", driver);
        command.env("RUSTC_WRAPPER", "");
    }

    let cargo_status = cargo_status(command)?;
    let (mut report, any_immediate_abort) = aggregate(&fragment_dir, &inv.roots)?;
    // The pre-flight guard reads the environment, and the strategy can also
    // arrive from the manifest (`cargo-features = ["panic-immediate-abort"]`
    // plus a profile), `config.toml`, or `--profile`. This one asks the
    // build itself: the driver recorded the strategy rustc actually used, so
    // a mixed configuration is caught however it was selected.
    //
    // Recorded into `selection_errors` before the report is written, not
    // just raised as a hard error afterward: `Report::is_success()` would
    // otherwise say `true` for a persisted `report.json` whose roots all
    // passed, even though those verdicts were produced against a sysroot
    // that was never rebuilt to match. A tool reading the file directly
    // (rather than this process's own exit code) needs the taint to be
    // in the artifact, not only on stderr.
    let mixed_sysroot = mixed_sysroot_panic_strategy(inv.build_std, any_immediate_abort);
    taint_report_for_mixed_sysroot(&mut report, mixed_sysroot);
    taint_report_for_failed_build(&mut report, cargo_status.success());
    // Persisted like the taints above, but as a warning: the exit status
    // stays 0 (a workspace may legitimately have no markers), yet a reader of
    // `report.json` alone must not take `roots: []` for a verified pass.
    if report.roots.is_empty() {
        report.warnings.push(ZERO_ROOTS_WARNING.to_owned());
    }
    report
        .write_to_file(&report_path)
        .context("failed to write final report.json")?;
    for error in &report.selection_errors {
        eprintln!(
            "{}: {error}",
            if inv.warn_only { "warning" } else { "error" }
        );
    }
    let failures = report
        .roots
        .iter()
        .filter(|root| !matches!(root.verdict, Verdict::Pass | Verdict::NotInstantiated))
        .count();
    eprintln!(
        "no_alloc: checked {} root instance(s), {failures} finding(s)",
        report.roots.len()
    );
    // 0 roots is a silent-pass hazard, not a clean pass: it exits 0 and
    // looks identical to "everything checked out" unless called out here.
    if report.roots.is_empty() {
        eprintln!("warning: {ZERO_ROOTS_WARNING}");
    }
    // The report is written first (above), so the evidence survives this
    // failure regardless of `--warn-only` — a mixed sysroot invalidates the
    // very mechanism `--warn-only` relies on to interpret results, so unlike
    // an ordinary finding it is never downgraded to a warning.
    ensure!(
        !mixed_sysroot,
        "this build compiled with `-Cpanic=immediate-abort` against the precompiled standard \
         library, which carries its own panic runtime — so the report's verdicts would claim \
         panic paths that lower to `abort` when std's do not. Pass `--immediate-abort`, which \
         applies the strategy to both halves, or `--build-std` if you are setting the \
         strategy yourself (see docs/iterators.md)"
    );
    if !cargo_status.success() {
        bail!("Cargo exited with {cargo_status}");
    }
    if !inv.warn_only && !report.is_success() {
        bail!("no_alloc check failed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    /// Serializes the tests that write a fake `rustc` and then execute it.
    ///
    /// Writing an executable and running it is not thread-safe against a
    /// *sibling* doing the same: the other test's `Command::spawn` inherits
    /// the still-open write fd, and the exec then fails with `ETXTBSY`
    /// ("Text file busy"). It reproduced about half the time here, as
    /// whichever test lost the race failing with a spawn error instead of
    /// the verdict it asserts on.
    static FAKE_RUSTC: std::sync::Mutex<()> = std::sync::Mutex::new(());

    use super::*;
    use no_alloc_report::{Environment, RootVerdict};
    use std::os::unix::fs::PermissionsExt;

    // Every pre-existing test wants an `Invocation`; only the new
    // help/version tests below care about the other `ParsedArgs` variants,
    // so they call `parse_args` directly instead of going through this.
    fn parse(args: &[&str]) -> Result<Invocation> {
        match parse_args(args.iter().map(|arg| (*arg).to_owned()))? {
            ParsedArgs::Run(inv) => Ok(inv),
            ParsedArgs::Help => bail!("expected an Invocation, got --help"),
            ParsedArgs::Version => bail!("expected an Invocation, got --version"),
        }
    }

    #[test]
    fn ld_library_path_without_existing_value() {
        assert_eq!(
            combined_ld_library_path(Path::new("/fake/sysroot/lib"), None),
            OsString::from("/fake/sysroot/lib")
        );
    }

    #[test]
    fn ld_library_path_prepended_to_existing_value() {
        assert_eq!(
            combined_ld_library_path(
                Path::new("/fake/sysroot/lib"),
                Some(std::ffi::OsStr::new("/other/lib"))
            ),
            OsString::from("/fake/sysroot/lib:/other/lib")
        );
    }

    #[test]
    fn ld_library_path_treats_empty_existing_value_as_absent() {
        assert_eq!(
            combined_ld_library_path(
                Path::new("/fake/sysroot/lib"),
                Some(std::ffi::OsStr::new(""))
            ),
            OsString::from("/fake/sysroot/lib")
        );
    }

    #[test]
    fn strips_actual_cargo_subcommand_argv() -> Result<()> {
        assert_eq!(
            parse(&["cargo-no-alloc", "no-alloc", "--", "build"])?.command,
            "build"
        );
        Ok(())
    }

    #[test]
    fn defaults_to_build() -> Result<()> {
        assert_eq!(parse(&["cargo-no-alloc"])?.command, "build");
        let with_options = parse(&["cargo-no-alloc", "--", "--release"])?;
        assert_eq!(with_options.command, "build");
        assert_eq!(with_options.cargo_args, ["--release"]);
        Ok(())
    }

    #[test]
    fn rejects_check_and_target_overrides() {
        assert!(parse(&["cargo-no-alloc", "--", "check"]).is_err());
        assert!(parse(&["cargo-no-alloc", "--", "build", "--target=x"]).is_err());
    }

    #[test]
    fn retains_test_runner_separator() -> Result<()> {
        let inv = parse(&["cargo-no-alloc", "--", "test", "case", "--", "--nocapture"])?;
        assert_eq!(inv.cargo_args, ["case", "--", "--nocapture"]);
        Ok(())
    }

    #[test]
    fn parses_checker_flags_and_roots() -> Result<()> {
        let inv = parse(&[
            "cargo-no-alloc",
            "no-alloc",
            "--warn-only",
            "--root",
            "z::f",
            "--root=a::f",
            "--",
            "test",
        ])?;
        assert!(inv.warn_only);
        assert_eq!(inv.roots, ["a::f", "z::f"]);
        assert_eq!(inv.command, "test");
        Ok(())
    }

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
    fn immediate_abort_implies_build_std() -> Result<()> {
        let inv = parse(&["cargo-no-alloc", "--immediate-abort", "--", "build"])?;
        assert!(inv.immediate_abort);
        // Rebuilding the sysroot is not optional here: the precompiled one
        // was built with a different panic strategy, and mixing them is
        // exactly what this flag exists to avoid.
        assert!(inv.build_std);
        // --build-std on its own must not turn the panic strategy on.
        let inv = parse(&["cargo-no-alloc", "--build-std", "--", "build"])?;
        assert!(inv.build_std);
        assert!(!inv.immediate_abort);
        Ok(())
    }

    #[test]
    fn immediate_abort_rejects_test_mode() {
        let error = parse(&["cargo-no-alloc", "--immediate-abort", "--", "test"])
            .expect_err("--immediate-abort -- test cannot build");
        // The point of rejecting at parse time is the message: rustc's own
        // failure arrives after a full sysroot rebuild and names an
        // unrelated-looking flag.
        assert!(error.to_string().contains("-- build"), "{error}");
        // Neither half is a problem on its own.
        assert!(parse(&["cargo-no-alloc", "--immediate-abort", "--", "build"]).is_ok());
        assert!(parse(&["cargo-no-alloc", "--", "test"]).is_ok());
    }

    #[test]
    fn immediate_abort_flag_is_appended_even_when_an_equal_flag_is_inherited() {
        // `-Cpanic` is last-wins, so "already present somewhere" is not the
        // same as "in effect": an inherited pair ending in `-Cpanic=unwind`
        // must not stop the checker's own copy from being appended last.
        let flags = |encoded: &str| {
            // SAFETY: see `orchestration_runs_with_fake_cargo_and_rustc` —
            // env mutation in this binary is serialized by `FAKE_RUSTC`.
            let _guard = FAKE_RUSTC
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let previous = std::env::var_os("CARGO_ENCODED_RUSTFLAGS");
            unsafe { std::env::set_var("CARGO_ENCODED_RUSTFLAGS", encoded) };
            let mut command = Command::new("true");
            add_required_rustflags(&mut command, true, &[]);
            let value = command
                .get_envs()
                .find(|(name, _)| *name == "CARGO_ENCODED_RUSTFLAGS")
                .and_then(|(_, value)| value)
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_default();
            unsafe {
                match previous {
                    Some(previous) => std::env::set_var("CARGO_ENCODED_RUSTFLAGS", previous),
                    None => std::env::remove_var("CARGO_ENCODED_RUSTFLAGS"),
                }
            }
            value
        };
        let outvoted = flags("-Cpanic=immediate-abort\u{1f}-Cpanic=unwind");
        let last = outvoted.split('\u{1f}').next_back().unwrap_or_default();
        assert_eq!(last, "-Cpanic=immediate-abort", "in `{outvoted}`");
    }

    #[test]
    fn aggregate_detects_immediate_abort_even_when_fragments_disagree() -> Result<()> {
        // The P1 scenario: a target-specific config selects immediate-abort
        // for the checked crate, while a wrapped host unit (a build script
        // with a checked root of its own) compiles under something else.
        // `Report::merge` correctly calls that disagreement `None` for
        // `report.json`'s own sake — but `aggregate`'s second return value
        // must still see the `ImmediateAbort` fragment, or the safety net in
        // `run` never fires.
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_aggregate_conflict_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&directory)?;
        let root = |name: &str, panic_strategy| RootVerdict {
            root: name.to_owned(),
            instance: name.to_owned(),
            verdict: Verdict::Pass,
            environment: Some(Environment {
                panic_strategy,
                opt_level: "No".into(),
                mir_opt_level: 1,
                overflow_checks: false,
                debug_assertions: false,
                target_triple: "x86_64-unknown-linux-gnu".into(),
                rustc_version: "1.99.0-nightly".into(),
                all_crates: false,
                build_std: true,
            }),
        };
        ReportFragment {
            report: Report {
                roots: vec![root("target::checked", PanicStrategy::ImmediateAbort)],
                panic_strategy: Some(PanicStrategy::ImmediateAbort),
                ..Report::default()
            },
            matched_root_specs: vec![],
        }
        .write_to_file(&directory.join("target.json"))?;
        ReportFragment {
            report: Report {
                roots: vec![root("build_script::checked", PanicStrategy::Unwind)],
                panic_strategy: Some(PanicStrategy::Unwind),
                ..Report::default()
            },
            matched_root_specs: vec![],
        }
        .write_to_file(&directory.join("host.json"))?;

        let (report, any_immediate_abort) = aggregate(&directory, &[])?;
        // The report itself is silent about the disagreement, as designed...
        assert_eq!(report.panic_strategy, None);
        assert_eq!(report.roots.len(), 2);
        assert_ne!(report.roots[0].environment, report.roots[1].environment);
        assert!(report.selection_errors.is_empty());
        // ...but the safety-net signal still catches the mix.
        assert!(any_immediate_abort);
        assert!(mixed_sysroot_panic_strategy(false, any_immediate_abort));

        std::fs::remove_dir_all(directory)?;
        Ok(())
    }

    #[test]
    fn taint_report_for_mixed_sysroot_flips_is_success() {
        // The scenario this exists for: a trivial root that would pass
        // under any panic strategy, so nothing about the traversal itself
        // objects — but the sysroot claim is untrustworthy, and a reader of
        // `report.json` alone (not this process's exit code) needs that to
        // show up in the file.
        let passing = Report {
            roots: vec![RootVerdict {
                root: "a".into(),
                instance: "a".into(),
                verdict: Verdict::Pass,
                environment: Some(Environment {
                    panic_strategy: PanicStrategy::ImmediateAbort,
                    opt_level: "No".into(),
                    mir_opt_level: 1,
                    overflow_checks: false,
                    debug_assertions: false,
                    target_triple: "x86_64-unknown-linux-gnu".into(),
                    rustc_version: "1.99.0-nightly".into(),
                    all_crates: false,
                    build_std: true,
                }),
            }],
            panic_strategy: Some(PanicStrategy::ImmediateAbort),
            ..Report::default()
        };
        assert!(passing.is_success());

        let mut tainted = passing.clone();
        taint_report_for_mixed_sysroot(&mut tainted, true);
        assert!(!tainted.is_success());
        assert_eq!(tainted.selection_errors.len(), 1);

        // Not tainted when the sysroot isn't mixed: a coherent report is
        // left exactly as `aggregate` produced it.
        let mut untouched = passing.clone();
        taint_report_for_mixed_sysroot(&mut untouched, false);
        assert_eq!(untouched, passing);
    }

    #[test]
    fn ambient_guard_accepts_build_std_without_the_immediate_abort_flag() {
        // The exact case this round found: `--build-std` alone, no
        // `--immediate-abort`, ambient flags already selecting the
        // strategy — coherent, and must not be pre-flight rejected.
        assert!(!ambient_immediate_abort_needs_build_std(
            true,
            Some("-Cpanic=immediate-abort"),
            None
        ));
        // Neither flag: still a real mix, still rejected.
        assert!(ambient_immediate_abort_needs_build_std(
            false,
            Some("-Cpanic=immediate-abort"),
            None
        ));
        // No ambient strategy at all: nothing to reject regardless of
        // `--build-std`.
        assert!(!ambient_immediate_abort_needs_build_std(false, None, None));
        assert!(!ambient_immediate_abort_needs_build_std(true, None, None));
    }

    #[test]
    fn mixed_sysroot_is_flagged_only_without_build_std() {
        // The dangerous case: crate compiled immediate-abort, sysroot left
        // precompiled (manifest/config selected the strategy, `--build-std`
        // was not passed).
        assert!(mixed_sysroot_panic_strategy(false, true));
        // `--build-std` rebuilds the sysroot under the same flags, so the
        // same observation is coherent once it is set.
        assert!(!mixed_sysroot_panic_strategy(true, true));
        // Nothing observed has nothing to be mixed with.
        assert!(!mixed_sysroot_panic_strategy(false, false));
    }

    #[test]
    fn immediate_abort_rejects_test_harness_target_selection() -> Result<()> {
        // `build --tests` reaches the same rustc failure as `-- test`, just
        // through target selection, and just as far into a sysroot rebuild.
        for selection in [vec!["--tests"], vec!["--all-targets"]] {
            let mut args = vec!["cargo-no-alloc", "--immediate-abort", "--", "build"];
            args.extend(selection.iter().copied());
            let error = parse(&args).expect_err(&format!("{selection:?} must be rejected"));
            assert!(error.to_string().contains("test-harness"), "{error}");
            // Only `--immediate-abort` has the problem; plain runs still build them.
            let mut plain = vec!["cargo-no-alloc", "--", "build"];
            plain.extend(selection.iter().copied());
            assert!(parse(&plain).is_ok(), "{selection:?} without the flag");
        }
        // Selections that name one target are *not* rejected: it may set
        // `harness = false` and build fine, and this wrapper does not read
        // per-target metadata to tell. The observed-strategy check after the
        // build is what protects the report either way.
        for selection in [
            vec!["--bench", "throughput"],
            vec!["--benches"],
            vec!["--test", "ui"],
        ] {
            let mut args = vec!["cargo-no-alloc", "--immediate-abort", "--", "build"];
            args.extend(selection.iter().copied());
            assert!(parse(&args).is_ok(), "{selection:?} must be allowed");
        }
        // Target selections that build no harness stay accepted.
        assert!(parse(&[
            "cargo-no-alloc",
            "--immediate-abort",
            "--",
            "build",
            "--lib"
        ])
        .is_ok());
        assert!(parse(&[
            "cargo-no-alloc",
            "--immediate-abort",
            "--",
            "build",
            "--examples"
        ])
        .is_ok());
        // Arguments after the test-runner separator are not target selection.
        parse(&[
            "cargo-no-alloc",
            "--immediate-abort",
            "--",
            "build",
            "--",
            "--tests",
        ])?;
        Ok(())
    }

    #[test]
    fn immediate_abort_rustflags_are_added_only_when_asked_for() {
        // `add_required_rustflags` reads the ambient RUSTFLAGS/
        // CARGO_ENCODED_RUSTFLAGS directly, so this needs the same isolation
        // `immediate_abort_flag_is_appended_even_when_an_equal_flag_is_inherited`
        // does: without it, either that test's transient mutation or a
        // developer's own shell exporting `-Cpanic=immediate-abort` could
        // leak in and fail the `without` assertions spuriously.
        let _guard = FAKE_RUSTC
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let saved = [
            ("RUSTFLAGS", std::env::var_os("RUSTFLAGS")),
            (
                "CARGO_ENCODED_RUSTFLAGS",
                std::env::var_os("CARGO_ENCODED_RUSTFLAGS"),
            ),
        ];
        // SAFETY: guarded by `FAKE_RUSTC`, restored before returning.
        unsafe {
            std::env::remove_var("RUSTFLAGS");
            std::env::remove_var("CARGO_ENCODED_RUSTFLAGS");
        }
        let flags = |immediate_abort| {
            let mut command = Command::new("true");
            add_required_rustflags(&mut command, immediate_abort, &[]);
            command
                .get_envs()
                .find(|(name, _)| *name == "RUSTFLAGS" || *name == "CARGO_ENCODED_RUSTFLAGS")
                .and_then(|(_, value)| value)
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        let without = flags(false);
        let with = flags(true);
        // SAFETY: same guard as above.
        unsafe {
            for (name, value) in saved {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
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
    fn rejects_malformed_checker_arguments() {
        assert!(parse(&["cargo-no-alloc", "--root"]).is_err());
        assert!(parse(&["cargo-no-alloc", "--root="]).is_err());
        assert!(parse(&["cargo-no-alloc", "--unknown"]).is_err());
        assert!(parse(&["cargo-no-alloc", "--", "run"]).is_err());
        assert!(parse(&["cargo-no-alloc", "--", "build", "--target", "x"]).is_err());
        assert!(parse(&["cargo-no-alloc", "--", "build", "--target-dir=x"]).is_err());
    }

    #[test]
    fn root_equals_and_direct_command_are_supported() -> Result<()> {
        let invocation = parse(&[
            "cargo-no-alloc",
            "--build-std",
            "--all-crates",
            "--root=z::f",
            "build",
        ])?;
        assert!(invocation.build_std);
        assert!(invocation.all_crates);
        assert_eq!(invocation.roots, ["z::f"]);
        Ok(())
    }

    #[test]
    fn help_flag_short_and_long_are_recognized() -> Result<()> {
        for args in [["cargo-no-alloc", "--help"], ["cargo-no-alloc", "-h"]] {
            assert!(matches!(
                parse_args(args.iter().map(|arg| (*arg).to_owned()))?,
                ParsedArgs::Help
            ));
        }
        Ok(())
    }

    #[test]
    fn version_flag_short_and_long_are_recognized() -> Result<()> {
        for args in [["cargo-no-alloc", "--version"], ["cargo-no-alloc", "-V"]] {
            assert!(matches!(
                parse_args(args.iter().map(|arg| (*arg).to_owned()))?,
                ParsedArgs::Version
            ));
        }
        Ok(())
    }

    #[test]
    fn help_flag_short_circuits_before_other_argument_errors() -> Result<()> {
        // A trailing bare `--root` (no path) would reject on its own if
        // parsing reached it — --help must return before that happens.
        assert!(matches!(
            parse_args(
                ["cargo-no-alloc", "--warn-only", "--help", "--root"]
                    .iter()
                    .map(|arg| (*arg).to_owned())
            )?,
            ParsedArgs::Help
        ));
        Ok(())
    }

    #[test]
    fn help_text_documents_every_flag_parse_args_accepts() {
        // Kept as a content check (rather than duplicating the text) so the
        // help output can't silently drift from what `parse_args` parses.
        for flag in [
            "--all-crates",
            "--build-std",
            "--immediate-abort",
            "--warn-only",
            "--root",
            "--help",
            "-h",
            "--version",
            "-V",
            "[build|test]",
        ] {
            assert!(HELP_TEXT.contains(flag), "help text missing `{flag}`");
        }
    }

    #[test]
    fn run_prints_help_and_version_without_touching_toolchain_or_cargo() {
        // Deliberately does not set RUSTC/CARGO to a fake: if either flag's
        // handling fell through to the real orchestration path, this would
        // fail by trying (and likely failing) to run the real `cargo`/`rustc`.
        assert!(run(["cargo-no-alloc".to_owned(), "--help".to_owned()]).is_ok());
        assert!(run(["cargo-no-alloc".to_owned(), "--version".to_owned()]).is_ok());
    }

    #[test]
    fn zero_roots_warning_names_the_two_likely_causes() {
        assert!(ZERO_ROOTS_WARNING.contains("--all-crates"));
        assert!(ZERO_ROOTS_WARNING.contains("0 root instances"));
        // Printed with a `warning:` prefix and persisted without one.
        assert!(!ZERO_ROOTS_WARNING.starts_with("warning:"));
    }

    #[test]
    fn verify_pinned_toolchain_accepts_the_pinned_version() -> Result<()> {
        let _guard = FAKE_RUSTC
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_toolchain_ok_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&directory)?;
        let fake_rustc = directory.join("rustc");
        std::fs::write(
            &fake_rustc,
            "#!/bin/sh\nprintf 'release: 1.99.0-nightly\\ncommit-hash: ad3d0bc14\\nhost: x86_64-unknown-linux-gnu\\n'\n",
        )?;
        let mut permissions = std::fs::metadata(&fake_rustc)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake_rustc, permissions)?;

        let result = verify_pinned_toolchain(&fake_rustc.to_string_lossy());
        std::fs::remove_dir_all(directory)?;
        result
    }

    #[test]
    fn verify_pinned_toolchain_rejects_a_mismatched_version() -> Result<()> {
        let _guard = FAKE_RUSTC
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_toolchain_bad_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&directory)?;
        let fake_rustc = directory.join("rustc");
        std::fs::write(
            &fake_rustc,
            "#!/bin/sh\nprintf 'release: 1.98.0-stable\\ncommit-hash: deadbeef00\\nhost: x86_64-unknown-linux-gnu\\n'\n",
        )?;
        let mut permissions = std::fs::metadata(&fake_rustc)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake_rustc, permissions)?;

        let error = verify_pinned_toolchain(&fake_rustc.to_string_lossy())
            .expect_err("mismatched toolchain must be rejected");
        assert!(error.to_string().contains("nightly-2026-08-01"));
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }

    #[test]
    fn aggregate_ignores_non_json_and_reports_unmatched() -> Result<()> {
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_aggregate_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&directory)?;
        std::fs::write(directory.join("ignore.txt"), "not json")?;
        ReportFragment {
            report: Report::default(),
            matched_root_specs: vec!["matched::root".into()],
        }
        .write_to_file(&directory.join("fragment.json"))?;

        let (report, any_immediate_abort) = aggregate(
            &directory,
            &["matched::root".into(), "missing::root".into()],
        )?;
        assert_eq!(report.selection_errors.len(), 1);
        assert!(report.selection_errors[0].contains("missing::root"));
        assert!(!any_immediate_abort);
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }

    #[test]
    fn aggregate_accepts_missing_fragment_directory() -> Result<()> {
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_missing_{}_{}",
            std::process::id(),
            line!()
        ));
        let (report, any_immediate_abort) = aggregate(&directory, &[])?;
        assert_eq!(report, Report::default());
        assert!(!any_immediate_abort);
        Ok(())
    }

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_owned()).collect()
    }

    #[test]
    fn clean_selection_args_forwards_profile_and_workspace_selection() {
        assert_eq!(
            clean_selection_args(&strings(&[
                "--release",
                "-p",
                "a",
                "--profile",
                "custom",
                "--manifest-path=other/Cargo.toml",
                "--config",
                "profile.custom.inherits='release'",
                "--features",
                "x",
            ])),
            strings(&[
                "--release",
                "--profile",
                "custom",
                "--manifest-path=other/Cargo.toml",
                "--config",
                "profile.custom.inherits='release'",
            ])
        );
        assert_eq!(
            clean_selection_args(&strings(&["-r", "--profile=bench", "--manifest-path", "m"])),
            strings(&["-r", "--profile=bench", "--manifest-path", "m"])
        );
    }

    #[test]
    fn clean_selection_args_stops_at_the_test_runner_separator() {
        assert_eq!(
            clean_selection_args(&strings(&["--lib", "--", "--release", "--profile", "x"])),
            Vec::<String>::new()
        );
        // A look-alike flag is not a profile selection.
        assert!(clean_selection_args(&strings(&["--profiler", "--releases"])).is_empty());
    }

    #[test]
    fn failed_build_taints_an_otherwise_passing_report() {
        let mut report = Report::default();
        assert!(report.is_success());
        taint_report_for_failed_build(&mut report, true);
        assert!(report.is_success());
        taint_report_for_failed_build(&mut report, false);
        assert!(!report.is_success());
    }

    #[test]
    fn failed_build_does_not_add_noise_to_an_already_failing_report() {
        let mut report = Report {
            selection_errors: vec!["already failing".into()],
            ..Report::default()
        };
        taint_report_for_failed_build(&mut report, false);
        assert_eq!(report.selection_errors, vec!["already failing".to_owned()]);
    }

    #[test]
    fn orchestration_runs_with_fake_cargo_and_rustc() -> Result<()> {
        // Also writes executables and runs them, so it shares the lock (see
        // `FAKE_RUSTC`) as well as being the only test that mutates the
        // process environment.
        let _guard = FAKE_RUSTC
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_orchestration_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&directory)?;
        let fake_rustc = directory.join("rustc");
        std::fs::write(
            &fake_rustc,
            // `run` now checks `-vV` (verify_pinned_toolchain) before it
            // ever reaches the `--print host-tuple`/`--print sysroot` calls
            // this script already answered.
            "#!/bin/sh\nif [ \"$1\" = \"-vV\" ]; then printf 'release: 1.99.0-nightly\\ncommit-hash: ad3d0bc14\\nhost: x86_64-unknown-linux-gnu\\n'; exit 0; fi\nif [ \"$1 $2\" = \"--print host-tuple\" ]; then echo x86_64-unknown-linux-gnu; exit 0; fi\nif [ \"$1 $2\" = \"--print sysroot\" ]; then echo /fake/sysroot; exit 0; fi\nexit 1\n",
        )?;
        let fake_cargo = directory.join("cargo");
        std::fs::write(
            &fake_cargo,
            "#!/bin/sh\nif [ \"$2\" = config ]; then echo \"error: config value is not set\" >&2; exit 101; fi\nif [ \"$1\" = clean ]; then echo \"$@\" >> clean.log; exit 0; fi\necho \"$NO_ALLOC_WARN_ONLY|$RUSTC|$NO_ALLOC_EXPECTED_RUSTC\" > env.log\nmkdir -p \"$NO_ALLOC_FRAGMENT_DIR\"\nprintf '%s\\n' '{\"report\":{\"roots\":[]},\"matched_root_specs\":[\"fake::root\"]}' > \"$NO_ALLOC_FRAGMENT_DIR/fake.json\"\nif [ \"$FAKE_CARGO_FAIL\" = 1 ]; then exit 1; fi\n",
        )?;
        for executable in [&fake_rustc, &fake_cargo] {
            let mut permissions = std::fs::metadata(executable)?.permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(executable, permissions)?;
        }

        let original_directory = std::env::current_dir()?;
        let variables = [
            "CARGO",
            "PATH",
            "RUSTC",
            "CARGO_BUILD_RUSTC",
            "CARGO_ENCODED_RUSTFLAGS",
            "NO_ALLOC_WARN_ONLY",
            "NO_ALLOC_ROOTS",
            "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER",
            "FAKE_CARGO_FAIL",
        ];
        let originals: Vec<_> = variables
            .iter()
            .map(|name| ((*name).to_owned(), std::env::var_os(name)))
            .collect();

        // SAFETY: this is the only test in this process that mutates the
        // process environment/current directory; the other tests are pure.
        unsafe {
            std::env::set_var("CARGO", &fake_cargo);
            // The checker resolves `rustc` from PATH (a `RUSTC` override is
            // refused), so the fake is made findable there instead.
            let mut path = directory.clone().into_os_string();
            if let Some(original) = std::env::var_os("PATH") {
                path.push(":");
                path.push(original);
            }
            std::env::set_var("PATH", path);
            std::env::remove_var("RUSTC");
            std::env::remove_var("CARGO_BUILD_RUSTC");
            std::env::set_var("CARGO_ENCODED_RUSTFLAGS", "--cfg=no_alloc_check");
            std::env::set_var("NO_ALLOC_WARN_ONLY", "1");
            std::env::set_var("NO_ALLOC_ROOTS", "fake::root");
            std::env::remove_var("RUSTC_WRAPPER");
            std::env::remove_var("RUSTC_WORKSPACE_WRAPPER");
        }
        std::env::set_current_dir(&directory)?;

        let result = run([
            "cargo-no-alloc".to_owned(),
            "--all-crates".to_owned(),
            "--build-std".to_owned(),
            "--root=fake::root".to_owned(),
            "--".to_owned(),
            "test".to_owned(),
            "--".to_owned(),
            "--list".to_owned(),
        ]);

        // A failed Cargo run must leave a report that does not read as a
        // success, and must clean the profile it is about to build. A
        // stale passing report from an earlier run must not survive either.
        let stale_report = directory.join("target/no-alloc/report.json");
        let failed_run = (|| -> Result<()> {
            // The zero-root run above must have persisted its warning, not
            // only printed it: the file is what other tools read.
            let first: Report = serde_json::from_reader(std::fs::File::open(&stale_report)?)?;
            ensure!(
                first
                    .warnings
                    .iter()
                    .any(|w| w.contains("0 root instances")),
                "a zero-root report must record the warning: {first:?}"
            );
            ensure!(first.is_success(), "a zero-root run still exits 0");
            let env_log = std::fs::read_to_string(directory.join("env.log"))?;
            ensure!(
                env_log.starts_with("1|"),
                "warn-only is passed as 1: {env_log}"
            );
            std::fs::write(&stale_report, r#"{"roots":[]}"#)?;
            // SAFETY: same single-mutator condition as above.
            unsafe {
                std::env::set_var("FAKE_CARGO_FAIL", "1");
                std::env::remove_var("NO_ALLOC_WARN_ONLY");
            }
            let failed = run(strings(&[
                "cargo-no-alloc",
                "--root=fake::root",
                "--",
                "build",
                "--release",
            ]));
            ensure!(failed.is_err(), "a failed Cargo run must fail the check");
            let report: Report = serde_json::from_reader(std::fs::File::open(&stale_report)?)?;
            ensure!(
                !report.is_success(),
                "report.json after a failed build must not read as a success"
            );
            // Never left unset: Cargo's `[env]` table would fill a hole.
            let env_log = std::fs::read_to_string(directory.join("env.log"))?;
            let fake = directory.join("rustc");
            ensure!(
                env_log.trim() == format!("0|{0}|{0}", fake.display()),
                "warn-only must be explicit and the validated rustc passed on: {env_log}"
            );
            let clean_log = std::fs::read_to_string(directory.join("clean.log"))?;
            ensure!(
                clean_log
                    .lines()
                    .last()
                    .is_some_and(|line| line.contains("--release")),
                "cargo clean must target the built profile: {clean_log}"
            );
            Ok(())
        })();

        std::env::set_current_dir(original_directory)?;
        for (name, value) in originals {
            // SAFETY: restoration occurs before this test returns, under the
            // same single-mutator condition described above.
            unsafe {
                if let Some(value) = value {
                    std::env::set_var(name, value);
                } else {
                    std::env::remove_var(name);
                }
            }
        }
        std::fs::remove_dir_all(directory)?;
        result?;
        failed_run
    }

    #[test]
    fn rustc_override_vars_are_named_in_the_refusal() {
        assert!(rustc_overrides_set(|_| false).is_empty());
        assert_eq!(rustc_overrides_set(|name| name == "RUSTC"), ["RUSTC"]);
        assert_eq!(
            rustc_overrides_set(|name| name == "CARGO_BUILD_RUSTC"),
            ["CARGO_BUILD_RUSTC"]
        );
    }

    #[test]
    fn run_refuses_a_custom_rustc_before_running_anything() -> Result<()> {
        let _guard = FAKE_RUSTC
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for variable in ["RUSTC", "CARGO_BUILD_RUSTC"] {
            let previous = std::env::var_os(variable);
            // SAFETY: env mutation in this binary is serialized by `FAKE_RUSTC`.
            unsafe { std::env::set_var(variable, "/nonexistent/wrapping-rustc") };
            let directory = std::env::temp_dir().join(format!(
                "cargo_no_alloc_rustc_override_{}_{variable}",
                std::process::id()
            ));
            std::fs::create_dir_all(&directory)?;
            let original_directory = std::env::current_dir()?;
            std::env::set_current_dir(&directory)?;
            let result = run(strings(&["cargo-no-alloc", "--", "build"]));
            std::env::set_current_dir(original_directory)?;
            unsafe {
                match previous {
                    Some(previous) => std::env::set_var(variable, previous),
                    None => std::env::remove_var(variable),
                }
            }
            std::fs::remove_dir_all(directory)?;
            let error = result.err().context("a custom rustc must be refused")?;
            // The nonexistent path proves the refusal came before any probe
            // tried to execute the substitute.
            let message = error.to_string();
            assert!(message.contains(variable), "{message}");
            assert!(message.contains("owns its compiler"), "{message}");
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
    fn rustc_config_key_warning_explains_the_toolchain_rustc() {
        let warning = wrapper_warning(
            "build.rustc",
            "build.rustc = \"./my-rustc\" # /p/.cargo/config.toml",
        );
        assert!(warning.contains("ignoring `build.rustc`"), "{warning}");
        assert!(warning.contains("toolchain's own rustc"), "{warning}");
        assert!(WRAPPER_CONFIG_KEYS.contains(&"build.rustc"));
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

    #[test]
    fn a_refused_run_leaves_a_report_that_is_not_a_success() {
        let report = rejected_report(vec!["b".to_owned(), "a".to_owned(), "a".to_owned()]);
        assert!(!report.is_success());
        assert_eq!(report.selection_errors, ["a", "b"]);
    }

    #[test]
    fn imported_config_flags_come_first_and_the_panic_strategy_stays_last() {
        let _guard = FAKE_RUSTC
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let saved = [
            ("RUSTFLAGS", std::env::var_os("RUSTFLAGS")),
            (
                "CARGO_ENCODED_RUSTFLAGS",
                std::env::var_os("CARGO_ENCODED_RUSTFLAGS"),
            ),
        ];
        // SAFETY: guarded by `FAKE_RUSTC`, restored before returning.
        unsafe {
            std::env::remove_var("RUSTFLAGS");
            std::env::remove_var("CARGO_ENCODED_RUSTFLAGS");
        }
        let imported = strings(&["--cfg", "heap", "-Cllvm-args=a b"]);
        let flags_for = |imported: &[String]| {
            let mut command = Command::new("true");
            add_required_rustflags(&mut command, true, imported);
            command
                .get_envs()
                .find(|(name, _)| *name == "CARGO_ENCODED_RUSTFLAGS")
                .and_then(|(_, value)| value)
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        let merged = flags_for(&imported);
        // With the user's own variable set, config is not consulted by Cargo,
        // so imported flags must not be mixed in.
        unsafe { std::env::set_var("RUSTFLAGS", "--cfg user") };
        let shadowed = command_rustflags_var(&imported);
        unsafe {
            for (name, value) in saved {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
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

    /// The `RUSTFLAGS` value `add_required_rustflags` exports.
    fn command_rustflags_var(imported: &[String]) -> String {
        let mut command = Command::new("true");
        add_required_rustflags(&mut command, false, imported);
        command
            .get_envs()
            .find(|(name, _)| *name == "RUSTFLAGS")
            .and_then(|(_, value)| value)
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}
