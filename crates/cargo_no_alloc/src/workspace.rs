//! The directory the checker owns inside the project, and the exclusive claim
//! on it.

use anyhow::{Context, Result};
use fs2::FileExt;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

pub(crate) struct WorkspacePaths {
    pub(crate) target_dir: PathBuf,
    pub(crate) fragment_dir: PathBuf,
    pub(crate) report_path: PathBuf,
}

/// Takes the exclusive lock, then deletes the previous run's `report.json`.
/// Both happen before anything that can fail for a reason of the run's own:
/// every exit after this would otherwise leave the previous report in place,
/// readable as this run's result, and two checker runs in one workspace would
/// otherwise share a target directory.
///
/// The returned `File` *is* the lock: the caller must keep it alive until the
/// run is over (bind it to a name, not `_`).
#[must_use = "dropping the File releases the lock"]
pub(crate) fn acquire_workspace(cwd: &Path) -> Result<(File, WorkspacePaths)> {
    let target_dir = cwd.join("target/no-alloc");
    let fragment_dir = target_dir.join("fragments");
    fs::create_dir_all(cwd.join("target"))?;
    let lock = File::create(cwd.join("target/no-alloc.lock"))?;
    lock.lock_exclusive()
        .context("failed to lock target/no-alloc.lock")?;
    let report_path = target_dir.join("report.json");
    match fs::remove_file(&report_path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(error).context("failed to remove the previous report.json");
        }
        _ => {}
    }
    Ok((
        lock,
        WorkspacePaths {
            target_dir,
            fragment_dir,
            report_path,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_workspace(label: &str) -> Result<PathBuf> {
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_workspace_{}_{label}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory)?;
        Ok(directory)
    }

    #[test]
    fn acquiring_removes_the_stale_report_and_holds_the_lock_until_dropped() -> Result<()> {
        let cwd = temp_workspace("lock")?;
        let stale = cwd.join("target/no-alloc/report.json");
        std::fs::create_dir_all(stale.parent().context("report has a parent")?)?;
        std::fs::write(&stale, r#"{"roots":[]}"#)?;

        let (lock, paths) = acquire_workspace(&cwd)?;
        assert_eq!(paths.report_path, stale);
        assert_eq!(paths.fragment_dir, cwd.join("target/no-alloc/fragments"));
        assert!(!stale.exists(), "a stale report must not survive the claim");

        // A second claimant is excluded for as long as the File lives...
        let rival = File::create(cwd.join("target/no-alloc.lock"))?;
        assert!(rival.try_lock_exclusive().is_err());
        // ...and only for that long.
        drop(lock);
        assert!(rival.try_lock_exclusive().is_ok());

        std::fs::remove_dir_all(cwd)?;
        Ok(())
    }

    #[test]
    fn a_missing_report_is_not_an_error() -> Result<()> {
        let cwd = temp_workspace("fresh")?;
        let (_lock, paths) = acquire_workspace(&cwd)?;
        assert!(!paths.report_path.exists());
        std::fs::remove_dir_all(cwd)?;
        Ok(())
    }
}
