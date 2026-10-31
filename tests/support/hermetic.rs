//! A checker invocation the developer's shell cannot influence, shared by the
//! UI harness (`tests/ui.rs`) and the throughput bench
//! (`benches/throughput.rs`, via `#[path]`). It lives in one place so every
//! caller sees the same scrubbed environment; a variable missing from one
//! copy would make that caller's results depend on whoever ran it.

use std::ffi::OsStr;
use std::process::Command;

/// Variables an ambient shell may set that change what the checker reports or
/// whether it runs at all.
const SCRUBBED: &[&str] = &[
    // Overrides the exit-status policy under test.
    "NO_ALLOC_WARN_ONLY",
    // cargo-no-alloc refuses to run when a rustc wrapper is already set,
    // because it needs that slot for its own driver. `cargo llvm-cov` sets
    // `RUSTC_WRAPPER` for instrumentation, which would otherwise make every
    // fixture bail before compiling anything; sccache users hit the same via
    // the `CARGO_BUILD_*` spellings, which would also shadow or add to what
    // the config fixtures assert on.
    "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER",
    "CARGO_BUILD_RUSTC_WRAPPER",
    "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
    // The checker refuses a custom compiler or target outright, and ambient
    // `RUSTFLAGS` would add to what the config fixtures exercise.
    "RUSTC",
    "CARGO_BUILD_RUSTC",
    "CARGO_BUILD_TARGET",
    "RUSTFLAGS",
    "CARGO_ENCODED_RUSTFLAGS",
    // Decides whether rustc's MIR inliner runs at opt-level >= 2 (it needs
    // incremental off) and which crate codegens an upstream non-generic
    // function; CI cache actions export `CARGO_INCREMENTAL=0`. Ambient state
    // must not decide a fixture's verdict.
    "CARGO_INCREMENTAL",
    // A hard error prints anyhow's backtrace when these are set — machine
    // paths, crate versions and all — which would put every stderr snapshot
    // at the mercy of the developer's shell. `RUST_LIB_BACKTRACE` turns the
    // same thing on by itself, so removing only `RUST_BACKTRACE` would leave
    // half the hole open.
    "RUST_BACKTRACE",
    "RUST_LIB_BACKTRACE",
];

/// `program` (the `cargo-no-alloc` binary, or `cargo` for the external
/// subcommand route) with the ambient environment scrubbed. A caller that
/// wants one of the scrubbed variables sets it afterwards, on purpose.
pub fn hermetic_checker(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    for name in SCRUBBED {
        command.env_remove(name);
    }
    command
        .env("NO_ALLOC_LOG", "off")
        // The expected.stderr snapshots are plain text; forcing color off
        // keeps the comparison stable regardless of the caller's own
        // `CARGO_TERM_COLOR` (CI sets `always`, which would otherwise leak
        // ANSI escapes into the diagnostic and break every snapshot).
        .env("CARGO_TERM_COLOR", "never");
    command
}
