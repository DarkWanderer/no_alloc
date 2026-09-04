//! Half of the F5 (soundness review) fixture for `--all-crates`: this crate
//! has no `#[no_alloc]` marker of its own, only a plain dependency on
//! `all_crates_dep`, whose marker `dep_allocates` (see its `lib.rs`) is
//! *not* in `all_crates_marker`'s own workspace. Under the default
//! (`RUSTC_WORKSPACE_WRAPPER`-only) invocation that marker is silently
//! never instrumented — `ui_matrix` runs this case exactly that way, and
//! `expected.json` records zero checked roots. The dedicated
//! `all_crates_flag_instruments_non_workspace_markers` test in `tests/ui.rs`
//! re-runs it with `--all-crates` and asserts the marker *is* caught there
//! (see ADR 0004's "This does not extend to `#[no_alloc]` markers
//! themselves" consequence, and the README's `--all-crates` section).

fn main() {
    println!("{}", all_crates_dep::dep_allocates());
}
