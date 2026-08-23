//! Fails the build early if the active toolchain isn't exactly the pinned
//! nightly with the components `no-alloc-driver` needs at runtime. The
//! driver itself locates `librustc_driver-*.so`/`libLLVM.so` via
//! `LD_LIBRARY_PATH`, set by `cargo_no_alloc::run` from the *invoking*
//! machine's sysroot — not baked in here — so a binary built on one machine
//! (e.g. CI) keeps working when run on another.

use anyhow::{bail, ensure, Context};
use std::path::Path;
use std::process::Command;

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
        version.contains("release: 1.99.0-nightly")
            && version.contains("commit-hash: ad3d0bc14")
            && host.contains("linux"),
        "cargo-no-alloc requires exactly nightly-2026-08-01 on Linux; found:\n{version}"
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
    Ok(())
}
