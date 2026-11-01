// Shared via `include!` between `build.rs` (checks the toolchain compiling
// this binary) and `src/toolchain.rs` (checks the toolchain of the project
// being analyzed, at `cargo no-alloc` run time). `build.rs` cannot depend on this
// crate's own lib, so `include!` is what keeps the pin from drifting apart
// between the two checks as it moves.

const EXPECTED_RELEASE: &str = "release: 1.101.0-nightly";
const EXPECTED_COMMIT_HASH: &str = "commit-hash: 92b5b8c87";
const EXPECTED_HOST_SUBSTR: &str = "linux";

/// `version` is the full `rustc -vV` output; `host` is the value of its
/// `host:` line.
fn is_pinned_toolchain(version: &str, host: &str) -> bool {
    version.contains(EXPECTED_RELEASE)
        && version.contains(EXPECTED_COMMIT_HASH)
        && host.contains(EXPECTED_HOST_SUBSTR)
}

/// Shared wording so a build-time failure (build.rs) and a run-time failure
/// (`cargo_no_alloc::run`) read identically to whoever hits either one.
fn toolchain_mismatch_message(version: &str) -> String {
    format!("cargo-no-alloc requires exactly nightly-2026-10-31 on Linux; found:\n{version}")
}

/// Runs `rustc args...` and returns its stdout. The one copy of the probe
/// sequence (spawn, exit status, UTF-8) for every question put to `rustc`, in
/// the build script and at run time alike. `not_utf8` names the probe in the
/// UTF-8 failure, since "which output was garbled" is what a reader needs.
fn rustc_stdout(rustc: &str, args: &[&str], not_utf8: &'static str) -> anyhow::Result<String> {
    use anyhow::{ensure, Context as _};
    let shown = args.join(" ");
    let output = std::process::Command::new(rustc)
        .args(args)
        .output()
        .with_context(|| format!("failed to run `{rustc} {shown}`"))?;
    ensure!(
        output.status.success(),
        "`{rustc} {shown}` exited with {}",
        output.status
    );
    String::from_utf8(output.stdout).context(not_utf8)
}

// Checks the toolchain of the project being analyzed, mirroring build.rs's
// compile-time check on the toolchain that built this binary. Without this,
// a project pinned to a different toolchain (e.g. via rust-toolchain.toml)
// resolves the wrong sysroot in `sysroot_lib_dir`, which otherwise only ever
// surfaces many steps later as a raw dynamic-linker error.
// `rustc` is threaded in explicitly (rather than read from the environment
// here) so this predicate can be unit tested against a fake rustc without
// touching process-wide env state.
pub(crate) fn verify_pinned_toolchain(rustc: &str) -> anyhow::Result<()> {
    use anyhow::{ensure, Context as _};
    let version = rustc_stdout(rustc, &["-vV"], "rustc version is not UTF-8")?;
    let host = version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .context("rustc did not report a host triple")?;
    ensure!(
        is_pinned_toolchain(&version, host),
        toolchain_mismatch_message(&version)
    );
    Ok(())
}

fn rustc_sysroot(rustc: &str) -> anyhow::Result<String> {
    Ok(
        rustc_stdout(rustc, &["--print", "sysroot"], "sysroot path is not valid UTF-8")?
            .trim()
            .to_owned(),
    )
}
