//! Links the driver against `librustc_driver-*.so` at runtime by baking the
//! pinned toolchain's sysroot lib dir into an rpath, so `no-alloc-driver`
//! works from outside a `cargo run` invocation (e.g. as `RUSTC_WORKSPACE_WRAPPER`).

use anyhow::{ensure, Context};
use std::process::Command;

fn main() -> anyhow::Result<()> {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
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

    println!("cargo::rustc-link-arg-bin=no-alloc-driver=-Wl,-rpath,{sysroot}/lib");
    println!("cargo::rerun-if-changed=build.rs");
    Ok(())
}
