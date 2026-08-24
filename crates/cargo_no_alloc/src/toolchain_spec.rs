// Shared via `include!` between `build.rs` (checks the toolchain compiling
// this binary) and `src/lib.rs` (checks the toolchain of the project being
// analyzed, at `cargo no-alloc` run time). `build.rs` cannot depend on this
// crate's own lib, so `include!` is what keeps the pin from drifting apart
// between the two checks as it moves.

const EXPECTED_RELEASE: &str = "release: 1.99.0-nightly";
const EXPECTED_COMMIT_HASH: &str = "commit-hash: ad3d0bc14";
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
    format!("cargo-no-alloc requires exactly nightly-2026-08-01 on Linux; found:\n{version}")
}
