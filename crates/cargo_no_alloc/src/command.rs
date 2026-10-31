//! The Cargo invocation that performs the checked build.

use crate::cargo_args::split_at_separator;
use crate::cli::{Config, Scope, StdMode};
use crate::rustflags::{add_required_rustflags, AmbientRustflags};
use crate::toolchain::{combined_ld_library_path, ValidatedRustc};
use crate::workspace::WorkspacePaths;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::Command;

/// The `NO_ALLOC_*` protocol between `cargo-no-alloc` and the driver, plus the
/// compiler both are told about.
///
/// Every variable is set on every run, never removed: a removed variable is a
/// hole Cargo's `[env]` table would fill. Booleans are exactly `"1"`/`"0"`,
/// and the driver treats only exactly `"1"` as true.
pub(crate) struct DriverEnv<'a> {
    pub(crate) fragment_dir: &'a Path,
    pub(crate) roots: &'a BTreeSet<String>,
    pub(crate) scope: Scope,
    pub(crate) std_mode: StdMode,
    pub(crate) warn_only: bool,
    pub(crate) rustc: &'a Path,
}

impl DriverEnv<'_> {
    pub(crate) fn apply(&self, command: &mut Command) {
        // Destructured without `..` on purpose: a new field cannot be added
        // without deciding here how it crosses the process boundary.
        let Self {
            fragment_dir,
            roots,
            scope,
            std_mode,
            warn_only,
            rustc,
        } = self;
        command.env("NO_ALLOC_FRAGMENT_DIR", fragment_dir);
        command.env(
            "NO_ALLOC_ROOTS",
            roots
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(","),
        );
        // Read back by the driver into each fragment's `Environment`: these
        // are invocation-level flags, not something `Session` exposes, so they
        // have to cross the process boundary as plain env vars like everything
        // else the driver needs from `cargo-no-alloc` itself.
        command.env("NO_ALLOC_ALL_CRATES", scope.driver_flag());
        command.env("NO_ALLOC_BUILD_STD", flag(std_mode.rebuilds_std()));
        command.env("NO_ALLOC_WARN_ONLY", flag(*warn_only));
        // Hand Cargo the rustc that was validated (`RUSTC` outranks
        // `build.rustc`), and tell the driver which one that is so it can
        // refuse to compile when anything else arrives as its argv[1].
        command.env("RUSTC", rustc);
        command.env("NO_ALLOC_EXPECTED_RUSTC", rustc);
    }
}

fn flag(value: bool) -> &'static str {
    if value {
        "1"
    } else {
        "0"
    }
}

/// Everything the build command is made of. Taking each input as a value
/// (including the inherited `LD_LIBRARY_PATH`) is what lets [`build_command`]
/// be tested without touching process state.
pub(crate) struct BuildInputs<'a> {
    pub(crate) cargo: &'a OsString,
    pub(crate) config: &'a Config,
    pub(crate) host: &'a str,
    pub(crate) paths: &'a WorkspacePaths,
    pub(crate) rustc: &'a ValidatedRustc,
    pub(crate) driver: &'a Path,
    pub(crate) sysroot_lib: &'a Path,
    pub(crate) inherited_ld_library_path: Option<&'a OsStr>,
    pub(crate) ambient: Option<&'a AmbientRustflags>,
}

pub(crate) fn build_command(inputs: &BuildInputs<'_>) -> Command {
    let BuildInputs {
        cargo,
        config,
        host,
        paths,
        rustc,
        driver,
        sysroot_lib,
        inherited_ld_library_path,
        ambient,
    } = inputs;
    let mut command = Command::new(cargo);
    command.arg(config.command.as_str());
    // The forced options go before a test-runner `--`, where Cargo reads them
    // and the runner does not.
    let (cargo_options, runner_args) = split_at_separator(&config.cargo_args);
    command.args(cargo_options);
    command
        .arg("--target")
        .arg(host)
        .arg("--target-dir")
        .arg(&paths.target_dir);
    if config.std_mode.rebuilds_std() {
        command.arg("-Zbuild-std");
    }
    command.args(runner_args);
    let (rustflags_var, rustflags) =
        add_required_rustflags(*ambient, config.std_mode.selects_immediate_abort());
    command.env(rustflags_var, rustflags);
    DriverEnv {
        fragment_dir: &paths.fragment_dir,
        roots: &config.roots,
        scope: config.scope,
        std_mode: config.std_mode,
        warn_only: config.warn_only,
        rustc: rustc.path(),
    }
    .apply(&mut command);
    command.env(
        "LD_LIBRARY_PATH",
        combined_ld_library_path(sysroot_lib, *inherited_ld_library_path),
    );
    for (name, value) in config.scope.wrapper_env(driver) {
        command.env(name, value);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{parse_args, ParsedArgs};
    use crate::rustflags::IMMEDIATE_ABORT_RUSTFLAGS;
    use crate::test_support::strings;
    use anyhow::{bail, Context, Result};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn config(
        args: &[&str],
        warn_only_env: Option<&str>,
        roots_env: Option<&str>,
    ) -> Result<Config> {
        match parse_args(args.iter().map(|arg| (*arg).to_owned()))? {
            ParsedArgs::Run(invocation) => {
                Ok(Config::resolve(invocation, warn_only_env, roots_env))
            }
            _ => bail!("expected a run"),
        }
    }

    struct Built {
        args: Vec<String>,
        env: BTreeMap<String, Option<String>>,
    }

    fn build(
        config: &Config,
        ambient: Option<&AmbientRustflags>,
        inherited_ld: Option<&OsStr>,
    ) -> Built {
        let paths = WorkspacePaths {
            target_dir: PathBuf::from("/w/target/no-alloc"),
            fragment_dir: PathBuf::from("/w/target/no-alloc/fragments"),
            report_path: PathBuf::from("/w/target/no-alloc/report.json"),
        };
        let rustc = ValidatedRustc::for_test(PathBuf::from("/t/bin/rustc"));
        let cargo = OsString::from("cargo");
        let command = build_command(&BuildInputs {
            cargo: &cargo,
            config,
            host: "x86_64-unknown-linux-gnu",
            paths: &paths,
            rustc: &rustc,
            driver: Path::new("/d/no-alloc-driver"),
            sysroot_lib: Path::new("/sys/lib"),
            inherited_ld_library_path: inherited_ld,
            ambient,
        });
        assert_eq!(command.get_program(), "cargo");
        Built {
            args: command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
            env: command
                .get_envs()
                .map(|(name, value)| {
                    (
                        name.to_string_lossy().into_owned(),
                        value.map(|value| value.to_string_lossy().into_owned()),
                    )
                })
                .collect(),
        }
    }

    impl Built {
        fn var(&self, name: &str) -> Option<&str> {
            self.env.get(name).and_then(|value| value.as_deref())
        }
    }

    #[test]
    fn forced_options_precede_the_test_runner_separator() -> Result<()> {
        let built = build(
            &config(
                &[
                    "cargo-no-alloc",
                    "--build-std",
                    "--",
                    "test",
                    "x",
                    "--",
                    "--list",
                ],
                None,
                None,
            )?,
            None,
            None,
        );
        assert_eq!(
            built.args,
            strings(&[
                "test",
                "x",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--target-dir",
                "/w/target/no-alloc",
                "-Zbuild-std",
                "--",
                "--list",
            ])
        );
        Ok(())
    }

    #[test]
    fn plain_builds_pass_no_std_option() -> Result<()> {
        let built = build(
            &config(&["cargo-no-alloc", "--", "build", "--release"], None, None)?,
            None,
            None,
        );
        assert_eq!(
            built.args,
            strings(&[
                "build",
                "--release",
                "--target",
                "x86_64-unknown-linux-gnu",
                "--target-dir",
                "/w/target/no-alloc",
            ])
        );
        Ok(())
    }

    /// The protocol variables are always present with exactly "0"/"1" —
    /// never absent, because a missing variable is a hole `[env]` would fill.
    #[test]
    fn protocol_variables_are_always_set_to_exactly_zero_or_one() -> Result<()> {
        let off = build(
            &config(&["cargo-no-alloc", "--", "build"], None, None)?,
            None,
            None,
        );
        for name in [
            "NO_ALLOC_ALL_CRATES",
            "NO_ALLOC_BUILD_STD",
            "NO_ALLOC_WARN_ONLY",
        ] {
            assert_eq!(off.var(name), Some("0"), "{name}");
        }
        assert_eq!(off.var("NO_ALLOC_ROOTS"), Some(""));

        let on = build(
            &config(
                &[
                    "cargo-no-alloc",
                    "--all-crates",
                    "--immediate-abort",
                    "--root=b::f",
                    "--",
                    "build",
                ],
                Some("1"),
                Some("a::f"),
            )?,
            None,
            None,
        );
        for name in [
            "NO_ALLOC_ALL_CRATES",
            "NO_ALLOC_BUILD_STD",
            "NO_ALLOC_WARN_ONLY",
        ] {
            assert_eq!(on.var(name), Some("1"), "{name}");
        }
        assert_eq!(on.var("NO_ALLOC_ROOTS"), Some("a::f,b::f"));
        assert_eq!(
            on.var("NO_ALLOC_FRAGMENT_DIR"),
            Some("/w/target/no-alloc/fragments")
        );
        Ok(())
    }

    #[test]
    fn the_validated_rustc_is_handed_to_cargo_and_to_the_driver() -> Result<()> {
        let built = build(
            &config(&["cargo-no-alloc", "--", "build"], None, None)?,
            None,
            None,
        );
        assert_eq!(built.var("RUSTC"), Some("/t/bin/rustc"));
        assert_eq!(built.var("NO_ALLOC_EXPECTED_RUSTC"), Some("/t/bin/rustc"));
        Ok(())
    }

    #[test]
    fn exactly_one_wrapper_slot_carries_the_driver_and_the_other_is_blanked() -> Result<()> {
        let workspace = build(
            &config(&["cargo-no-alloc", "--", "build"], None, None)?,
            None,
            None,
        );
        assert_eq!(
            workspace.var("RUSTC_WORKSPACE_WRAPPER"),
            Some("/d/no-alloc-driver")
        );
        assert_eq!(workspace.var("RUSTC_WRAPPER"), Some(""));
        let all = build(
            &config(
                &["cargo-no-alloc", "--all-crates", "--", "build"],
                None,
                None,
            )?,
            None,
            None,
        );
        assert_eq!(all.var("RUSTC_WRAPPER"), Some("/d/no-alloc-driver"));
        assert_eq!(all.var("RUSTC_WORKSPACE_WRAPPER"), Some(""));
        Ok(())
    }

    #[test]
    fn the_sysroot_lib_comes_first_in_the_library_path() -> Result<()> {
        let config = config(&["cargo-no-alloc", "--", "build"], None, None)?;
        let bare = build(&config, None, None);
        assert_eq!(bare.var("LD_LIBRARY_PATH"), Some("/sys/lib"));
        let extended = build(&config, None, Some(OsStr::new("/other")));
        assert_eq!(extended.var("LD_LIBRARY_PATH"), Some("/sys/lib:/other"));
        Ok(())
    }

    #[test]
    fn the_flags_variable_follows_the_ambient_source_and_the_panic_strategy_stays_last(
    ) -> Result<()> {
        let plain = AmbientRustflags::Plain("--cfg user".into());
        let built = build(
            &config(
                &["cargo-no-alloc", "--immediate-abort", "--", "build"],
                None,
                None,
            )?,
            Some(&plain),
            None,
        );
        assert_eq!(built.var("CARGO_ENCODED_RUSTFLAGS"), None);
        let flags = built.var("RUSTFLAGS").context("RUSTFLAGS must be set")?;
        assert!(flags.starts_with("--cfg user "), "{flags}");
        assert!(
            flags.ends_with(&IMMEDIATE_ABORT_RUSTFLAGS.join(" ")),
            "{flags}"
        );

        let encoded = AmbientRustflags::Encoded("--cfg=x".into());
        let built = build(
            &config(&["cargo-no-alloc", "--", "build"], None, None)?,
            Some(&encoded),
            None,
        );
        assert_eq!(built.var("RUSTFLAGS"), None);
        assert!(built
            .var("CARGO_ENCODED_RUSTFLAGS")
            .is_some_and(|flags| flags.starts_with("--cfg=x\u{1f}")));
        Ok(())
    }
}
