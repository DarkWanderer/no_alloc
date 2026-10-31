mod cargo_args;
mod cargo_config;
mod cli;
mod command;
mod outcome;
mod preflight;
mod rustflags;
#[cfg(test)]
mod test_support;
mod toolchain;
mod workspace;

pub use rustflags::REQUIRED_RUSTFLAGS;

use anyhow::{ensure, Context, Result};
use cargo_args::clean_selection_args;
use cli::{parse_args, Config, ParsedArgs, Scope, HELP_TEXT};
use command::{build_command, BuildInputs};
use outcome::{
    aggregate, finalize, mixed_sysroot_panic_strategy, refuse, refuse_configuration, Aggregated,
};
use preflight::{preflight, CargoConfig};
use rustflags::{reject_ambient_immediate_abort, AmbientRustflags};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitStatus};
use toolchain::{
    find_driver, host_triple, reject_wrapper_env, sysroot_lib_dir, validate_toolchain,
    ValidatedRustc,
};
use workspace::{acquire_workspace, WorkspacePaths};

const CACHE_DIR_TAG: &str = "Signature: 8a477f597d28d172789f06886806bc55\n\
# This file is a cache directory tag created by cargo.\n\
# For information about cache directory tags see https://bford.info/cachedir/\n";

fn cargo_status(mut command: Command) -> Result<ExitStatus> {
    command.status().context("failed to spawn Cargo")
}

/// The target directory exists, with Cargo's safety tag, before Cargo first
/// sees this custom target, so `cargo clean` can manage it on this and later
/// runs.
fn ensure_target_dir(paths: &WorkspacePaths) -> Result<()> {
    fs::create_dir_all(&paths.target_dir)?;
    fs::write(paths.target_dir.join("CACHEDIR.TAG"), CACHE_DIR_TAG)?;
    Ok(())
}

fn clear_checker_artifacts(
    cargo: &OsString,
    config: &Config,
    target_dir: &Path,
    host: &str,
    rustc: &Path,
) -> Result<()> {
    let mut clean = Command::new(cargo);
    clean.arg("clean");
    // `cargo clean --target` probes the compiler too, so a `build.rustc` from
    // config would otherwise run here, before the build's own override.
    clean.env("RUSTC", rustc);
    if config.scope == Scope::Workspace {
        clean.arg("--workspace");
    }
    clean
        .args(clean_selection_args(&config.cargo_args))
        .arg("--target")
        .arg(host)
        .arg("--target-dir")
        .arg(target_dir);
    let status = cargo_status(clean)?;
    ensure!(status.success(), "Cargo clean exited with {status}");
    Ok(())
}

/// Cleans what a stale cache could hide behind: the checker's artifacts, so
/// Cargo cannot find a unit fresh and skip the driver (and with it that unit's
/// roots), and the previous run's fragments, which would otherwise be merged
/// into this run's report.
fn prepare_target(
    cargo: &OsString,
    config: &Config,
    host: &str,
    rustc: &ValidatedRustc,
    paths: &WorkspacePaths,
) -> Result<()> {
    clear_checker_artifacts(cargo, config, &paths.target_dir, host, rustc.path())?;
    if paths.fragment_dir.exists() {
        fs::remove_dir_all(&paths.fragment_dir)?;
    }
    fs::create_dir_all(&paths.fragment_dir)?;
    Ok(())
}

/// Orchestration only; each step that decides something lives in a function
/// of its own. The order is part of the contract (and unit-tested where it
/// can be): the lock is taken and the stale report removed before anything
/// fallible; every refusal after that leaves a failing `report.json`; the
/// toolchain is validated before any probe runs; and the report is written
/// before the exit status is decided.
pub fn run(args: impl IntoIterator<Item = String>) -> Result<()> {
    let config = match parse_args(args)? {
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
        ParsedArgs::Run(invocation) => Config::resolve(
            invocation,
            env::var("NO_ALLOC_WARN_ONLY").ok().as_deref(),
            env::var("NO_ALLOC_ROOTS").ok().as_deref(),
        ),
    };
    let cwd = env::current_dir().context("failed to determine current directory")?;
    // Named, not `_`: the File is the lock, and must outlive everything below.
    let (_lock, paths) = acquire_workspace(&cwd)?;
    // Every refusal from here on goes through `refuse` and friends, which need
    // only the report path (known once the lock is held and the stale report
    // is gone), so even the earliest one leaves a failing report.
    //
    // Read before anything else so the flags are decoded once and every later
    // use sees the same value; a non-UTF-8 one is a refusal, not a lossy guess.
    let env_rustflags = AmbientRustflags::from_env(
        env::var_os("CARGO_ENCODED_RUSTFLAGS"),
        env::var_os("RUSTFLAGS"),
    )
    .map_err(|error| refuse(&paths.report_path, error.to_string()))?;
    let rustc = validate_toolchain(&paths.report_path)?;
    reject_wrapper_env(&paths.report_path)?;

    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let host = host_triple(rustc.as_str())?;
    ensure_target_dir(&paths)?;

    let found = preflight(
        &CargoConfig::new(&cargo, &config.cargo_args),
        &host,
        env_rustflags,
        |warning| eprintln!("warning: {warning}"),
    )?;
    refuse_configuration(&paths.report_path, found.rejections)?;
    reject_ambient_immediate_abort(&paths.report_path, config.std_mode, found.ambient.as_ref())?;

    prepare_target(&cargo, &config, &host, &rustc, &paths)?;
    let driver = find_driver()?;
    let sysroot_lib = sysroot_lib_dir(rustc.as_str())?;
    let command = build_command(&BuildInputs {
        cargo: &cargo,
        config: &config,
        host: &host,
        paths: &paths,
        rustc: &rustc,
        driver: &driver,
        sysroot_lib: &sysroot_lib,
        inherited_ld_library_path: env::var_os("LD_LIBRARY_PATH").as_deref(),
        ambient: found.ambient.as_ref(),
    });

    let cargo_status = cargo_status(command)?;
    let Aggregated {
        report,
        any_immediate_abort,
    } = aggregate(&paths.fragment_dir, &config.roots)?;
    let mixed_sysroot = mixed_sysroot_panic_strategy(config.std_mode, any_immediate_abort);
    finalize(
        report,
        &paths.report_path,
        cargo_status,
        mixed_sysroot,
        config.warn_only,
    )
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
