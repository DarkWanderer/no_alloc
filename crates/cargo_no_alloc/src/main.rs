//! `cargo-no-alloc`: sets up the wrapper env described in the Invocation
//! section of docs/design.md, then shells out to `cargo build`/`cargo test`.
//! Never `cargo check` — the mono item graph is a codegen artifact.

use anyhow::{bail, ensure, Context, Result};
use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Flags the driver needs; kept in sync with `no_alloc_driver::REQUIRED_FLAGS`.
/// Duplicated rather than shared because the driver also re-adds these itself
/// when invoked directly (`RUSTC=no-alloc-driver`) — see docs/design.md.
const RUSTFLAGS: &[&str] = &[
    "--cfg=no_alloc_check",
    "--check-cfg=cfg(no_alloc_check)",
    "-Zcrate-attr=feature(register_tool)",
    "-Zcrate-attr=register_tool(no_alloc_tool)",
    "-Zalways-encode-mir",
];

const TARGET_DIR: &str = "target/no-alloc";

struct Invocation {
    build_std: bool,
    all_crates: bool,
    cargo_args: Vec<String>,
}

fn parse_args(args: impl Iterator<Item = String>) -> Invocation {
    let mut build_std = false;
    let mut all_crates = false;
    let mut rest = Vec::new();
    let mut args = args.peekable();

    while let Some(arg) = args.peek() {
        match arg.as_str() {
            "--build-std" => {
                build_std = true;
                args.next();
            }
            "--all-crates" => {
                all_crates = true;
                args.next();
            }
            "--" => {
                args.next();
                break;
            }
            _ => break,
        }
    }
    rest.extend(args);

    if rest.is_empty() {
        rest.push("build".to_string());
    }

    Invocation {
        build_std,
        all_crates,
        cargo_args: rest,
    }
}

fn host_triple() -> Result<String> {
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let output = Command::new(&rustc)
        .args(["--print", "host-tuple"])
        .output()
        .with_context(|| format!("failed to run `{rustc} --print host-tuple`"))?;
    ensure!(
        output.status.success(),
        "`{rustc} --print host-tuple` exited with {}",
        output.status
    );
    Ok(String::from_utf8(output.stdout)
        .context("host-tuple output is not valid UTF-8")?
        .trim()
        .to_string())
}

/// The driver binary is installed alongside `cargo-no-alloc` (same target
/// dir, same `cargo install` bin dir); fall back to PATH lookup so
/// `cargo build --workspace && cargo no-alloc` still works without
/// installing anything.
fn find_driver() -> Result<PathBuf> {
    let exe = env::current_exe().context("failed to resolve current_exe")?;
    let sibling = exe
        .parent()
        .context("current_exe has no parent directory")?
        .join("no-alloc-driver");
    if sibling.is_file() {
        return Ok(sibling);
    }
    Ok(PathBuf::from("no-alloc-driver"))
}

/// `CARGO_ENCODED_RUSTFLAGS` joins flags with `\x1f` (unlike `RUSTFLAGS`,
/// which splits on whitespace and would mangle a flag value containing one).
fn encode_rustflags(flags: &[&str]) -> String {
    flags.join("\x1f")
}

fn run() -> Result<()> {
    let inv = parse_args(env::args().skip(1));
    let host = host_triple()?;
    let driver = find_driver()?;

    let mut cmd = Command::new("cargo");
    cmd.args(&inv.cargo_args);
    cmd.arg("--target").arg(&host);
    cmd.arg("--target-dir").arg(TARGET_DIR);
    if inv.build_std {
        cmd.arg("-Zbuild-std");
    }
    cmd.env("CARGO_ENCODED_RUSTFLAGS", encode_rustflags(RUSTFLAGS));
    if inv.all_crates {
        cmd.env("RUSTC_WRAPPER", &driver);
    } else {
        cmd.env("RUSTC_WORKSPACE_WRAPPER", &driver);
    }

    tracing::info!(?cmd, driver = %driver.display(), "running cargo");

    let status = cmd.status().context("failed to spawn cargo")?;
    if !status.success() {
        bail!("cargo exited with {status}");
    }
    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("NO_ALLOC_LOG")
                .unwrap_or_else(|_| "cargo_no_alloc=info".into()),
        )
        .init();

    run()
}
