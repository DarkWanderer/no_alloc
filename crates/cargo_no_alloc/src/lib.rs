mod cargo_args;
mod cargo_config;
mod cli;
mod outcome;
mod rustflags;
#[cfg(test)]
mod test_support;
mod toolchain;

pub use rustflags::REQUIRED_RUSTFLAGS;

use anyhow::{bail, ensure, Context, Result};
use cargo_args::{clean_selection_args, split_at_separator};
use cargo_config::{
    build_rustc_rejection, build_target_rejection, cargo_config_json, config_override_args,
    configured_wrappers, parse_build_targets, per_package_target_rejection, reserved_env_keys,
    reserved_env_rejection, workspace_per_package_targets,
};
use cli::{parse_args, Invocation, ParsedArgs, Scope, HELP_TEXT};
use fs2::FileExt;
use no_alloc_report::parse_root_spec;
use outcome::{
    aggregate, mixed_sysroot_panic_strategy, print_summary, refuse, rejected_report,
    taint_report_for_failed_build, taint_report_for_mixed_sysroot, Aggregated, ZERO_ROOTS_WARNING,
};
use rustflags::{
    add_required_rustflags, ambient_immediate_abort_needs_build_std, config_rustflags,
    AmbientRustflags, ConfigRustflags,
};
use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::path::Path;
use std::process::{Command, ExitStatus};
use toolchain::{
    combined_ld_library_path, find_driver, host_triple, reject_rustc_override, resolve_rustc,
    sysroot_lib_dir, verify_pinned_toolchain,
};

const CACHE_DIR_TAG: &str = "Signature: 8a477f597d28d172789f06886806bc55\n\
# This file is a cache directory tag created by cargo.\n\
# For information about cache directory tags see https://bford.info/cachedir/\n";

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
    if inv.scope == Scope::Workspace {
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
    // Every refusal from here on goes through `refuse`, which needs only the
    // report path (known right above, once the lock is held and the stale
    // report is gone), so even the earliest one leaves a failing report.
    //
    // Read before anything else so the flags are decoded once and every later
    // use sees the same value; a non-UTF-8 one is a refusal, not a lossy guess.
    let env_rustflags = AmbientRustflags::from_env(
        env::var_os("CARGO_ENCODED_RUSTFLAGS"),
        env::var_os("RUSTFLAGS"),
    )
    .map_err(|error| refuse(&report_path, error.to_string()))?;
    // Checked before anything else touches Cargo: a project pinned to the
    // wrong toolchain otherwise fails much further down, in
    // `sysroot_lib_dir`, as a raw dynamic-linker error instead of this
    // actionable message.
    //
    // The override check comes first so that no probe ever runs a substitute
    // compiler, and the probes below use the same resolved path the build is
    // handed.
    reject_rustc_override(&report_path)?;
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
        return Err(refuse(
            &report_path,
            "existing RUSTC_WRAPPER/RUSTC_WORKSPACE_WRAPPER configuration is incompatible with cargo-no-alloc"
                .to_owned(),
        ));
    }
    if let Ok(existing) = env::var("NO_ALLOC_ROOTS") {
        inv.roots.extend(parse_root_spec(&existing));
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
    // (`build.rustc` is not: see `WRAPPER_CONFIG_KEYS`.)
    for warning in configured_wrappers(&cargo, &config_args)? {
        eprintln!("warning: {warning}");
    }
    let mut rejections = Vec::new();
    if let Some(json) = cargo_config_json(&cargo, &config_args, "build.rustc")? {
        // An empty value is Cargo's "unset".
        let value = json.trim();
        if value != "\"\"" && !value.is_empty() {
            rejections.push(build_rustc_rejection(value));
        }
    }
    if let Some(json) = cargo_config_json(&cargo, &config_args, "build.target")? {
        let targets = parse_build_targets(&json)?;
        if !targets.is_empty() {
            rejections.push(build_target_rejection(&targets));
        }
    }
    let per_package_targets = workspace_per_package_targets(&cargo, &inv.cargo_args)?;
    if !per_package_targets.is_empty() {
        rejections.push(per_package_target_rejection(&per_package_targets));
    }
    if let Some(json) = cargo_config_json(&cargo, &config_args, "env")? {
        let keys = reserved_env_keys(&json)?;
        if !keys.is_empty() {
            rejections.push(reserved_env_rejection(&keys));
        }
    }
    // Cargo ignores config rustflags entirely once either variable is set
    // (even to an empty value), so the caller's environment is already what
    // plain `cargo build` would use and there is nothing to import.
    let mut ambient = env_rustflags;
    if ambient.is_none() {
        match config_rustflags(&cargo, &config_args, &host)? {
            ConfigRustflags::Import(flags) => {
                if !flags.is_empty() {
                    tracing::info!(?flags, "importing rustflags from Cargo config");
                }
                ambient = AmbientRustflags::from_config(flags);
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
    // Gating on the immediate-abort option here instead rejected that
    // legitimate combination before ever reaching the build.
    if ambient_immediate_abort_needs_build_std(inv.std_mode, ambient.as_ref()) {
        return Err(refuse(
            &report_path,
            "the environment's or Cargo config's Rust flags already select `-Cpanic=immediate-abort`, but neither \
             `--immediate-abort` nor `--build-std` was passed. That strategy only means what the \
             report says it means if the standard library is rebuilt with it too; pass \
             `--immediate-abort` (which does both) or `--build-std` (if you are setting the \
             strategy yourself)"
                .to_owned(),
        ));
    }

    clear_checker_artifacts(&cargo, &inv, &target_dir, &host, &rustc_path)?;
    if fragment_dir.exists() {
        fs::remove_dir_all(&fragment_dir)?;
    }
    fs::create_dir_all(&fragment_dir)?;

    let mut command = Command::new(&cargo);
    command.arg(inv.command.as_str());
    let (cargo_options, runner_args) = split_at_separator(&inv.cargo_args);
    command.args(cargo_options);
    command
        .arg("--target")
        .arg(&host)
        .arg("--target-dir")
        .arg(&target_dir);
    if inv.std_mode.rebuilds_std() {
        command.arg("-Zbuild-std");
    }
    command.args(runner_args);
    let (rustflags_var, rustflags) =
        add_required_rustflags(ambient.as_ref(), inv.std_mode.selects_immediate_abort());
    command.env(rustflags_var, rustflags);
    command.env("NO_ALLOC_FRAGMENT_DIR", &fragment_dir);
    command.env(
        "NO_ALLOC_ROOTS",
        inv.roots.iter().cloned().collect::<Vec<_>>().join(","),
    );
    // Read back by the driver into each fragment's `Environment` (F4): these
    // are invocation-level flags, not something `Session` exposes, so they
    // have to cross the process boundary as plain env vars like everything
    // else the driver needs from `cargo-no-alloc` itself.
    command.env("NO_ALLOC_ALL_CRATES", inv.scope.driver_flag());
    command.env(
        "NO_ALLOC_BUILD_STD",
        if inv.std_mode.rebuilds_std() {
            "1"
        } else {
            "0"
        },
    );
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
    for (name, value) in inv.scope.wrapper_env(&driver) {
        command.env(name, value);
    }

    let cargo_status = cargo_status(command)?;
    let Aggregated {
        mut report,
        any_immediate_abort,
    } = aggregate(&fragment_dir, &inv.roots)?;
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
    let mixed_sysroot = mixed_sysroot_panic_strategy(inv.std_mode, any_immediate_abort);
    taint_report_for_mixed_sysroot(&mut report, mixed_sysroot);
    taint_report_for_failed_build(&mut report, cargo_status.success());
    // Persisted like the taints above, but as a warning: the exit status
    // stays 0 (a workspace may legitimately have no markers), yet a reader of
    // `report.json` alone must not take `roots: []`, or a list of roots that
    // were never instantiated, for a verified pass.
    if !report.checked_an_instance() {
        report.warnings.push(ZERO_ROOTS_WARNING.to_owned());
    }
    report
        .write_to_file(&report_path)
        .context("failed to write final report.json")?;
    print_summary(&report, inv.warn_only);
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
    use super::*;
    use crate::test_support::{strings, FAKE_RUSTC};
    use no_alloc_report::Report;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn run_prints_help_and_version_without_touching_toolchain_or_cargo() {
        // Deliberately does not set RUSTC/CARGO to a fake: if either flag's
        // handling fell through to the real orchestration path, this would
        // fail by trying (and likely failing) to run the real `cargo`/`rustc`.
        assert!(run(["cargo-no-alloc".to_owned(), "--help".to_owned()]).is_ok());
        assert!(run(["cargo-no-alloc".to_owned(), "--version".to_owned()]).is_ok());
    }

    /// `NAME=value` lines as printed by `env`, split at the first `=`.
    fn parse_env_log(log: &str) -> std::collections::BTreeMap<String, String> {
        log.lines()
            .filter_map(|line| line.split_once('='))
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect()
    }

    #[test]
    fn orchestration_runs_with_fake_cargo_and_rustc() -> Result<()> {
        // Also writes executables and runs them, so it shares the lock (see
        // `FAKE_RUSTC`). Other tests mutate the process environment too, but
        // only under that same lock, so none can observe this one's values.
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
        // The build invocation logs its whole argv and every variable the
        // wrapper is responsible for, so the contract with Cargo (argument
        // layout, the always-set `NO_ALLOC_*` protocol, which wrapper slot is
        // used and which blanked) is asserted on rather than assumed. Only
        // variables that are set show up in `env.log`; an empty one prints as
        // `NAME=`.
        std::fs::write(
            &fake_cargo,
            r#"#!/bin/sh
if [ "$1" = metadata ]; then echo '{"packages":[],"workspace_members":[]}'; exit 0; fi
if [ "$2" = config ]; then echo "error: config value is not set" >&2; exit 101; fi
if [ "$1" = clean ]; then echo "$@" >> clean.log; exit 0; fi
printf '%s\n' "$@" > argv.log
env | grep -E '^(NO_ALLOC_[A-Z_]*|RUSTC|RUSTC_WRAPPER|RUSTC_WORKSPACE_WRAPPER|RUSTFLAGS|CARGO_ENCODED_RUSTFLAGS|LD_LIBRARY_PATH)=' | sort > env.log
mkdir -p "$NO_ALLOC_FRAGMENT_DIR"
printf '%s\n' '{"report":{"roots":[]},"matched_root_specs":["fake::root"]}' > "$NO_ALLOC_FRAGMENT_DIR/fake.json"
if [ "$FAKE_CARGO_FAIL" = 1 ]; then exit 1; fi
"#,
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
            "RUSTFLAGS",
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

        // SAFETY: every test that mutates the process environment or current
        // directory holds `FAKE_RUSTC`, and this one holds it for its whole
        // body; the remaining tests are pure.
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
            std::env::remove_var("RUSTFLAGS");
            std::env::set_var("NO_ALLOC_WARN_ONLY", "1");
            std::env::set_var("NO_ALLOC_ROOTS", "fake::root");
            std::env::remove_var("RUSTC_WRAPPER");
            std::env::remove_var("RUSTC_WORKSPACE_WRAPPER");
        }
        std::env::set_current_dir(&directory)?;
        // After the chdir, so it is the same spelling `run` sees.
        let cwd = std::env::current_dir()?;

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
            let fake = directory.join("rustc");
            let target_dir = cwd.join("target/no-alloc");

            // Run 1: `--all-crates --build-std`, test mode with a runner
            // separator, warn-only and roots inherited from the environment.
            let argv = std::fs::read_to_string(directory.join("argv.log"))?;
            let argv: Vec<&str> = argv.lines().collect();
            ensure!(
                argv == [
                    "test",
                    "--target",
                    "x86_64-unknown-linux-gnu",
                    "--target-dir",
                    &target_dir.to_string_lossy(),
                    "-Zbuild-std",
                    "--",
                    "--list",
                ],
                "forced options go before the runner separator: {argv:?}"
            );
            let env = parse_env_log(&std::fs::read_to_string(directory.join("env.log"))?);
            let get = |name: &str| env.get(name).map(String::as_str);
            ensure!(get("NO_ALLOC_WARN_ONLY") == Some("1"), "{env:?}");
            ensure!(get("NO_ALLOC_ALL_CRATES") == Some("1"), "{env:?}");
            ensure!(get("NO_ALLOC_BUILD_STD") == Some("1"), "{env:?}");
            ensure!(get("NO_ALLOC_ROOTS") == Some("fake::root"), "{env:?}");
            ensure!(
                get("NO_ALLOC_FRAGMENT_DIR")
                    == Some(&*target_dir.join("fragments").to_string_lossy()),
                "{env:?}"
            );
            ensure!(get("RUSTC") == Some(&*fake.to_string_lossy()), "{env:?}");
            ensure!(
                get("NO_ALLOC_EXPECTED_RUSTC") == Some(&*fake.to_string_lossy()),
                "{env:?}"
            );
            // `--all-crates` takes the unconditional wrapper slot and blanks
            // the other; the blank must be present, not merely absent.
            ensure!(
                get("RUSTC_WRAPPER").is_some_and(|driver| driver.ends_with("no-alloc-driver")),
                "{env:?}"
            );
            ensure!(get("RUSTC_WORKSPACE_WRAPPER") == Some(""), "{env:?}");
            ensure!(
                get("LD_LIBRARY_PATH").is_some_and(|path| path.starts_with("/fake/sysroot/lib")),
                "{env:?}"
            );
            // The caller's encoded flags are extended, never replaced, and
            // the required flags are added exactly once each.
            ensure!(get("RUSTFLAGS").is_none(), "{env:?}");
            let encoded = get("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
            let flags: Vec<&str> = encoded.split('\x1f').collect();
            ensure!(flags[0] == "--cfg=no_alloc_check", "{flags:?}");
            for required in REQUIRED_RUSTFLAGS {
                ensure!(
                    flags.iter().filter(|flag| *flag == required).count() == 1,
                    "`{required}` must appear exactly once in {flags:?}"
                );
            }
            ensure!(flags.len() == REQUIRED_RUSTFLAGS.len(), "{flags:?}");

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
            let argv = std::fs::read_to_string(directory.join("argv.log"))?;
            let argv: Vec<&str> = argv.lines().collect();
            ensure!(
                argv == [
                    "build",
                    "--release",
                    "--target",
                    "x86_64-unknown-linux-gnu",
                    "--target-dir",
                    &target_dir.to_string_lossy(),
                ],
                "no `-Zbuild-std` and no separator without them: {argv:?}"
            );
            let env = parse_env_log(&std::fs::read_to_string(directory.join("env.log"))?);
            let get = |name: &str| env.get(name).map(String::as_str);
            ensure!(
                get("NO_ALLOC_WARN_ONLY") == Some("0"),
                "warn-only must be explicit: {env:?}"
            );
            ensure!(get("NO_ALLOC_ALL_CRATES") == Some("0"), "{env:?}");
            ensure!(get("NO_ALLOC_BUILD_STD") == Some("0"), "{env:?}");
            ensure!(get("RUSTC") == Some(&*fake.to_string_lossy()), "{env:?}");
            ensure!(
                get("NO_ALLOC_EXPECTED_RUSTC") == Some(&*fake.to_string_lossy()),
                "the validated rustc is passed on: {env:?}"
            );
            // Mirror of run 1: workspace-only mode uses the workspace slot.
            ensure!(get("RUSTC_WRAPPER") == Some(""), "{env:?}");
            ensure!(
                get("RUSTC_WORKSPACE_WRAPPER")
                    .is_some_and(|driver| driver.ends_with("no-alloc-driver")),
                "{env:?}"
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
    fn run_refuses_a_custom_rustc_before_running_anything() -> Result<()> {
        let _guard = FAKE_RUSTC
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for variable in ["RUSTC", "CARGO_BUILD_RUSTC"] {
            let (result, report) = run_refused(
                variable,
                &[(variable, OsString::from("/nonexistent/wrapping-rustc"))],
            )?;
            let error = result.err().context("a custom rustc must be refused")?;
            // The nonexistent path proves the refusal came before any probe
            // tried to execute the substitute.
            let message = error.to_string();
            assert!(message.contains(variable), "{message}");
            assert!(message.contains("owns its compiler"), "{message}");
            // Persisted too, and in place of the stale passing report that
            // was on disk when the run started.
            assert!(!report.is_success(), "{report:?}");
            assert_eq!(report.selection_errors, [message]);
        }
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn run_refuses_non_utf8_rustflags_and_records_it() -> Result<()> {
        use std::os::unix::ffi::OsStringExt;
        let _guard = FAKE_RUSTC
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let (result, report) = run_refused(
            "non_utf8_flags",
            &[(
                "CARGO_ENCODED_RUSTFLAGS",
                OsString::from_vec(vec![b'-', 0xff]),
            )],
        )?;
        let message = result
            .err()
            .context("a non-UTF-8 value must be refused")?
            .to_string();
        assert!(message.contains("not valid UTF-8"), "{message}");
        assert!(!report.is_success(), "{report:?}");
        assert_eq!(report.selection_errors, [message]);
        Ok(())
    }

    /// Runs `cargo no-alloc -- build` in a fresh directory that already holds
    /// a stale *passing* `report.json`, with `env` set, and returns the run's
    /// result together with the report left on disk. The caller must hold
    /// `FAKE_RUSTC`: the environment and cwd are process-wide.
    fn run_refused(label: &str, env: &[(&str, OsString)]) -> Result<(Result<()>, Report)> {
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_refused_{}_{label}",
            std::process::id()
        ));
        std::fs::create_dir_all(directory.join("target/no-alloc"))?;
        std::fs::write(
            directory.join("target/no-alloc/report.json"),
            r#"{"roots":[]}"#,
        )?;
        let originals: Vec<_> = env
            .iter()
            .map(|(name, _)| ((*name).to_owned(), std::env::var_os(name)))
            .collect();
        let original_directory = std::env::current_dir()?;
        // SAFETY: callers hold `FAKE_RUSTC`, which serializes every test that
        // touches the process environment or current directory.
        unsafe {
            for (name, value) in env {
                std::env::set_var(name, value);
            }
        }
        std::env::set_current_dir(&directory)?;
        let result = run(strings(&["cargo-no-alloc", "--", "build"]));
        std::env::set_current_dir(original_directory)?;
        unsafe {
            for (name, value) in originals {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
        let report = serde_json::from_reader(std::fs::File::open(
            directory.join("target/no-alloc/report.json"),
        )?)?;
        std::fs::remove_dir_all(directory)?;
        Ok((result, report))
    }
}
