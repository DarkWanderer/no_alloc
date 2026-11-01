//! The toolchain of the project being analyzed: which `rustc` is used, that it
//! is the pinned one, and where the driver and its shared libraries live.

use crate::outcome::refuse;
use anyhow::{bail, Context, Result};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

// Shared with `build.rs` so the pin can't drift between the build-time
// check (the toolchain compiling this binary) and the run-time check below
// (the toolchain of the project being analyzed).
include!("toolchain_spec.rs");

pub(crate) fn host_triple(rustc: &str) -> Result<String> {
    Ok(rustc_stdout(
        rustc,
        &["--print", "host-tuple"],
        "host-tuple output is not UTF-8",
    )?
    .trim()
    .to_owned())
}

/// Variables that make Cargo compile with a program other than the
/// toolchain's own `rustc`. The driver links its own rustc and ignores the
/// binary Cargo hands it, so a script behind one of these (adding `--cfg`
/// flags, say) shapes the shipped build but never the checked one.
pub(crate) const RUSTC_OVERRIDE_VARS: &[&str] = &["RUSTC", "CARGO_BUILD_RUSTC"];

/// The override variables `is_set` reports, in declaration order. Taking the
/// predicate keeps this testable without touching process-wide state.
pub(crate) fn rustc_overrides_set(is_set: impl Fn(&str) -> bool) -> Vec<&'static str> {
    RUSTC_OVERRIDE_VARS
        .iter()
        .copied()
        .filter(|name| is_set(name))
        .collect()
}

pub(crate) fn reject_rustc_override(report_path: &Path) -> Result<()> {
    let set = rustc_overrides_set(|name| env::var_os(name).is_some());
    if set.is_empty() {
        return Ok(());
    }
    Err(refuse(
        report_path,
        format!(
            "{} is set, but the checker compiles with the toolchain's own rustc and cannot honour a \
             substitute: a custom rustc would shape the shipped build and not the checked one. \
             Unset it; the checker owns its compiler",
            set.join("/")
        ),
    ))
}

/// The `rustc` Cargo would find on `PATH`, as an absolute path. Not
/// canonicalized: under rustup this is a symlink to the `rustup` proxy, which
/// only dispatches on the name it was invoked by.
///
/// The probes and the build both use this one path (the build is handed it as
/// `RUSTC`), so what was validated is what runs.
pub(crate) fn resolve_rustc() -> Result<PathBuf> {
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

/// A `rustc` that has been through, in this order: the override refusal,
/// resolution from `PATH`, and the pinned-toolchain check. [`validate_toolchain`]
/// is the only way to get one, so holding it is the proof that every probe and
/// the build itself use the compiler that was validated. It replaces a
/// `String` and a `PathBuf` that had to be kept in step by hand.
#[derive(Debug)]
pub(crate) struct ValidatedRustc {
    path: PathBuf,
    text: String,
}

impl ValidatedRustc {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }

    /// Skips validation so tests of what is *done with* a validated compiler
    /// need no real toolchain.
    #[cfg(test)]
    pub(crate) fn for_test(path: PathBuf) -> Self {
        let text = path.to_string_lossy().into_owned();
        Self { path, text }
    }

    /// The last step: `path` must be UTF-8 (it is passed to probes as text)
    /// and the pinned toolchain.
    fn verify(path: PathBuf) -> Result<Self> {
        let text = path
            .to_str()
            .context("the path of `rustc` is not valid UTF-8")?
            .to_owned();
        verify_pinned_toolchain(&text)?;
        Ok(Self { path, text })
    }
}

/// Checked before anything else touches Cargo: a project pinned to the wrong
/// toolchain otherwise fails much further down, in `sysroot_lib_dir`, as a raw
/// dynamic-linker error instead of an actionable message.
///
/// The override check comes first so that no probe ever runs a substitute
/// compiler, and the probes after it use the same resolved path the build is
/// handed.
pub(crate) fn validate_toolchain(report_path: &Path) -> Result<ValidatedRustc> {
    reject_rustc_override(report_path)?;
    ValidatedRustc::verify(resolve_rustc()?)
}

/// `RUSTC_WRAPPER`/`RUSTC_WORKSPACE_WRAPPER` in the environment: the driver
/// needs that slot, and unlike a configured wrapper (which is blanked) an
/// environment one is the caller saying "run my program here", so it is refused.
pub(crate) fn reject_wrapper_env(report_path: &Path) -> Result<()> {
    if env::var_os("RUSTC_WRAPPER").is_some() || env::var_os("RUSTC_WORKSPACE_WRAPPER").is_some() {
        return Err(refuse(
            report_path,
            "existing RUSTC_WRAPPER/RUSTC_WORKSPACE_WRAPPER configuration is incompatible with cargo-no-alloc"
                .to_owned(),
        ));
    }
    Ok(())
}

pub(crate) fn sysroot_lib_dir(rustc: &str) -> Result<PathBuf> {
    Ok(PathBuf::from(rustc_sysroot(rustc)?).join("lib"))
}

// `no-alloc-driver` dynamically links `librustc_driver-*.so`/`libLLVM.so`,
// which live in the sysroot rather than next to the binary. A binary built
// on one machine (e.g. CI) and run on another can't rely on a path baked in
// at build time, so this is resolved fresh from whatever `rustc` is active
// for the invoking project and threaded through as `LD_LIBRARY_PATH`.
pub(crate) fn combined_ld_library_path(
    sysroot_lib: &Path,
    existing: Option<&std::ffi::OsStr>,
) -> OsString {
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

pub(crate) fn find_driver() -> Result<PathBuf> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FAKE_RUSTC;
    use std::os::unix::fs::PermissionsExt;

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
            "#!/bin/sh\nprintf 'release: 1.101.0-nightly\\ncommit-hash: 92b5b8c87\\nhost: x86_64-unknown-linux-gnu\\n'\n",
        )?;
        let mut permissions = std::fs::metadata(&fake_rustc)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake_rustc, permissions)?;

        let result = verify_pinned_toolchain(&fake_rustc.to_string_lossy());
        std::fs::remove_dir_all(directory)?;
        result
    }

    /// What a `ValidatedRustc` carries is exactly the path that was probed, in
    /// both spellings, so nothing downstream can use a different compiler.
    #[test]
    fn a_validated_rustc_is_the_path_that_passed_the_probe() -> Result<()> {
        let _guard = FAKE_RUSTC
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let directory =
            std::env::temp_dir().join(format!("cargo_no_alloc_validated_{}", std::process::id()));
        std::fs::create_dir_all(&directory)?;
        let fake_rustc = directory.join("rustc");
        std::fs::write(
            &fake_rustc,
            "#!/bin/sh\nprintf 'release: 1.101.0-nightly\\ncommit-hash: 92b5b8c87\\nhost: x86_64-unknown-linux-gnu\\n'\n",
        )?;
        let mut permissions = std::fs::metadata(&fake_rustc)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake_rustc, permissions)?;

        let validated = ValidatedRustc::verify(fake_rustc.clone());
        std::fs::remove_dir_all(directory)?;
        let validated = validated?;
        assert_eq!(validated.path(), fake_rustc);
        assert_eq!(Path::new(validated.as_str()), fake_rustc);
        // A compiler that fails the probe never becomes one.
        assert!(ValidatedRustc::verify(PathBuf::from("/nonexistent/rustc")).is_err());
        Ok(())
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
        assert!(error.to_string().contains("nightly-2026-10-31"));
        std::fs::remove_dir_all(directory)?;
        Ok(())
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
}
