//! Fails the build early if the active toolchain isn't exactly the pinned
//! nightly with the components `no-alloc-driver` needs at runtime. The
//! driver itself locates `librustc_driver-*.so`/`libLLVM.so` via
//! `LD_LIBRARY_PATH`, set by `cargo_no_alloc::run` from the *invoking*
//! machine's sysroot — not baked in here — so a binary built on one machine
//! (e.g. CI) keeps working when run on another.

use anyhow::{bail, ensure, Context};
use std::path::Path;
use std::process::Command;

// Shared with `src/lib.rs` so the pin can't drift between the build-time
// check (this file) and the run-time check (the project being analyzed).
include!("src/toolchain_spec.rs");

fn main() -> anyhow::Result<()> {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let version = Command::new(&rustc)
        .arg("-vV")
        .output()
        .with_context(|| format!("failed to run `{rustc} -vV`"))?;
    ensure!(
        version.status.success(),
        "`{rustc} -vV` exited with {}",
        version.status
    );
    let version = String::from_utf8(version.stdout).context("rustc version is not UTF-8")?;
    let host = version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .context("rustc did not report a host triple")?;
    ensure!(
        is_pinned_toolchain(&version, host),
        toolchain_mismatch_message(&version)
    );

    let output = Command::new(&rustc)
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
        .to_string();

    let sysroot_path = Path::new(&sysroot);
    ensure!(
        sysroot_path
            .join("lib/rustlib/rustc-src/rust/compiler")
            .is_dir(),
        "nightly-2026-08-01 is missing the rustc-dev component"
    );
    ensure!(
        sysroot_path.join("lib/rustlib/src/rust/library").is_dir(),
        "nightly-2026-08-01 is missing the rust-src component"
    );
    let driver_found = std::fs::read_dir(sysroot_path.join("lib"))?
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("librustc_driver-")
        });
    if !driver_found {
        bail!("nightly-2026-08-01 sysroot does not contain librustc_driver");
    }

    println!("cargo::rerun-if-changed=build.rs");
    // include!()'d above, so a change here must also invalidate this script.
    println!("cargo::rerun-if-changed=src/toolchain_spec.rs");
    Ok(())
}
