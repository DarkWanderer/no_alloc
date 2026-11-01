//! Fails the build early if the active toolchain isn't exactly the pinned
//! nightly with the components `no-alloc-driver` needs at runtime. The
//! driver itself locates `librustc_driver-*.so`/`libLLVM.so` via
//! `LD_LIBRARY_PATH`, set by `cargo_no_alloc::run` from the *invoking*
//! machine's sysroot — not baked in here — so a binary built on one machine
//! (e.g. CI) keeps working when run on another.

use anyhow::{bail, ensure};
use std::path::Path;

// Shared with `src/toolchain.rs` so the pin, and the way `rustc` is probed
// for it, can't drift between the build-time check (this file) and the
// run-time check (the project being analyzed).
include!("src/toolchain_spec.rs");

fn main() -> anyhow::Result<()> {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    verify_pinned_toolchain(&rustc)?;
    let sysroot = rustc_sysroot(&rustc)?;

    let sysroot_path = Path::new(&sysroot);
    ensure!(
        sysroot_path
            .join("lib/rustlib/rustc-src/rust/compiler")
            .is_dir(),
        "nightly-2026-10-31 is missing the rustc-dev component"
    );
    ensure!(
        sysroot_path.join("lib/rustlib/src/rust/library").is_dir(),
        "nightly-2026-10-31 is missing the rust-src component"
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
        bail!("nightly-2026-10-31 sysroot does not contain librustc_driver");
    }

    println!("cargo::rerun-if-changed=build.rs");
    // include!()'d above, so a change here must also invalidate this script.
    println!("cargo::rerun-if-changed=src/toolchain_spec.rs");
    Ok(())
}
