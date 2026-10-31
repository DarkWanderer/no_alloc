//! Configuration the environment-variable checks cannot see, read through
//! Cargo itself. A run that cannot be made faithful is collected here as a
//! rejection, to be recorded in a persisted report (not only refused on
//! stderr), so a consumer reading `report.json` never finds a clean file for a
//! build that was not checked.

use crate::cargo_config::{
    build_rustc_rejection, build_target_rejection, cargo_config_json, config_override_args,
    configured_wrappers, parse_build_targets, per_package_target_rejection, reserved_env_keys,
    reserved_env_rejection, workspace_per_package_targets,
};
use crate::rustflags::{config_rustflags, AmbientRustflags, ConfigRustflags};
use anyhow::Result;
use std::ffi::OsString;

/// How to ask Cargo about the build: the binary, the build's own arguments,
/// and the `--config` overrides among them, which feed the same configuration
/// the build reads and so must reach every query.
pub(crate) struct CargoConfig<'a> {
    cargo: &'a OsString,
    cargo_args: &'a [String],
    config_args: Vec<String>,
}

impl<'a> CargoConfig<'a> {
    pub(crate) fn new(cargo: &'a OsString, cargo_args: &'a [String]) -> Self {
        Self {
            cargo,
            cargo_args,
            config_args: config_override_args(cargo_args),
        }
    }

    fn json(&self, key: &str) -> Result<Option<String>> {
        cargo_config_json(self.cargo, &self.config_args, key)
    }
}

pub(crate) struct Preflight {
    /// Reasons the build cannot be checked faithfully; any makes the run a
    /// refusal.
    pub(crate) rejections: Vec<String>,
    /// The flags the build will run under: the caller's environment if it set
    /// any, else the (non-empty) ones imported from Cargo config.
    pub(crate) ambient: Option<AmbientRustflags>,
}

/// Queries Cargo's configuration. `env_rustflags` is what the environment
/// already supplied; Cargo ignores config rustflags entirely once either
/// variable is set (even to an empty value), so then there is nothing to
/// import. `warn` receives non-fatal findings as they are made, rather than
/// at the end, so one printed before a later query fails is not lost.
pub(crate) fn preflight(
    config: &CargoConfig<'_>,
    host: &str,
    env_rustflags: Option<AmbientRustflags>,
    mut warn: impl FnMut(String),
) -> Result<Preflight> {
    // Neutralised at build time by blanking the wrapper variables, so only a
    // warning. (`build.rustc` is not: see `WRAPPER_CONFIG_KEYS`.)
    for warning in configured_wrappers(config.cargo, &config.config_args)? {
        warn(warning);
    }
    let mut rejections = Vec::new();
    if let Some(json) = config.json("build.rustc")? {
        // An empty value is Cargo's "unset".
        let value = json.trim();
        if value != "\"\"" && !value.is_empty() {
            rejections.push(build_rustc_rejection(value));
        }
    }
    if let Some(json) = config.json("build.target")? {
        let targets = parse_build_targets(&json)?;
        if !targets.is_empty() {
            rejections.push(build_target_rejection(&targets));
        }
    }
    let per_package_targets = workspace_per_package_targets(config.cargo, config.cargo_args)?;
    if !per_package_targets.is_empty() {
        rejections.push(per_package_target_rejection(&per_package_targets));
    }
    if let Some(json) = config.json("env")? {
        let keys = reserved_env_keys(&json)?;
        if !keys.is_empty() {
            rejections.push(reserved_env_rejection(&keys));
        }
    }
    let mut ambient = env_rustflags;
    if ambient.is_none() {
        match config_rustflags(config.cargo, &config.config_args, host)? {
            ConfigRustflags::Import(flags) => {
                if !flags.is_empty() {
                    tracing::info!(?flags, "importing rustflags from Cargo config");
                }
                ambient = AmbientRustflags::from_config(flags);
            }
            ConfigRustflags::Unsupported(reason) => rejections.push(reason),
        }
    }
    Ok(Preflight {
        rejections,
        ambient,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{strings, FAKE_RUSTC};
    use std::os::unix::fs::PermissionsExt;

    /// A fake `cargo` that answers `config get <key>` from a small table, so
    /// the whole sequence runs without a process-wide environment: the binary
    /// is passed in, not read from `CARGO`.
    fn fake_cargo(label: &str, answers: &str) -> Result<(std::path::PathBuf, std::path::PathBuf)> {
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_preflight_{}_{label}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory)?;
        let cargo = directory.join("cargo");
        std::fs::write(
            &cargo,
            format!(
                r#"#!/bin/sh
if [ "$1" = metadata ]; then echo '{{"packages":[],"workspace_members":[]}}'; exit 0; fi
for last; do :; done
case "$last" in
{answers}
  *) echo "error: config value \`$last\` is not set" >&2; exit 101;;
esac
"#
            ),
        )?;
        let mut permissions = std::fs::metadata(&cargo)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&cargo, permissions)?;
        Ok((directory, cargo))
    }

    fn run_preflight(
        label: &str,
        answers: &str,
        env_rustflags: Option<AmbientRustflags>,
    ) -> Result<(Preflight, Vec<String>)> {
        let _guard = FAKE_RUSTC
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let (directory, cargo) = fake_cargo(label, answers)?;
        let cargo = cargo.into_os_string();
        let args = strings(&["build"]);
        let mut warnings = Vec::new();
        let result = preflight(
            &CargoConfig::new(&cargo, &args),
            "x86_64-unknown-linux-gnu",
            env_rustflags,
            |warning| warnings.push(warning),
        );
        std::fs::remove_dir_all(directory)?;
        Ok((result?, warnings))
    }

    #[test]
    fn a_clean_configuration_yields_nothing() -> Result<()> {
        let (found, warnings) = run_preflight("clean", "", None)?;
        assert!(found.rejections.is_empty());
        assert_eq!(found.ambient, None);
        assert!(warnings.is_empty());
        Ok(())
    }

    #[test]
    fn configuration_the_checker_cannot_honour_is_collected_not_fatal() -> Result<()> {
        let (found, warnings) = run_preflight(
            "rejections",
            r#"  build.rustc) echo '"/x/rustc"';;
  build.target) echo '"thumbv7em-none-eabihf"';;
  env) echo '{"NO_ALLOC_WARN_ONLY":"1"}';;
  build.rustc-wrapper) echo 'build.rustc-wrapper = "sccache" # /h/.cargo/config.toml';;"#,
            None,
        )?;
        assert_eq!(found.rejections.len(), 3, "{:?}", found.rejections);
        assert!(found.rejections[0].contains("build.rustc"));
        assert!(found.rejections[1].contains("build.target"));
        assert!(found.rejections[2].contains("NO_ALLOC_WARN_ONLY"));
        // A configured wrapper is neutralised, so it only warns.
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("build.rustc-wrapper"), "{warnings:?}");
        Ok(())
    }

    #[test]
    fn config_rustflags_are_imported_only_when_the_environment_set_none() -> Result<()> {
        let answers = r#"  build.rustflags) echo '["--cfg","heap"]';;"#;
        let (found, _) = run_preflight("import", answers, None)?;
        assert_eq!(
            found.ambient,
            Some(AmbientRustflags::FromConfig(strings(&["--cfg", "heap"])))
        );
        // Cargo would ignore the config here, so it is not even consulted.
        let user = AmbientRustflags::Plain("--cfg user".into());
        let (found, _) = run_preflight("shadowed", answers, Some(user.clone()))?;
        assert_eq!(found.ambient, Some(user));
        Ok(())
    }

    #[test]
    fn cfg_keyed_rustflags_are_a_rejection() -> Result<()> {
        let (found, _) = run_preflight(
            "cfg_tables",
            r#"  target) echo '{"cfg(unix)":{"rustflags":["--cfg","x"]}}';;"#,
            None,
        )?;
        assert_eq!(found.rejections.len(), 1);
        assert!(found.rejections[0].contains("cfg(unix)"));
        assert_eq!(found.ambient, None);
        Ok(())
    }
}
