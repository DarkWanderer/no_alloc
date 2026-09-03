//! Regression fixture for F2 (soundness review), pairing with
//! `panic_abort`: the guarantee's panic exclusion is drawn by *terminator
//! shape*, not by "does this path panic". `panic_abort::root`'s
//! `values[index]` lowers to an `Assert` terminator, which has no MIR
//! callee to follow and is out of scope under `panic=abort` (see ADR
//! 0003) — so it passes. `.unwrap()` here lowers to an ordinary `Call`
//! terminator to `Option::unwrap`/`unwrap_failed`, a real edge this
//! traversal *does* follow — those have no MIR body (foreign/bodiless), so
//! it rejects, even under the identical `panic = "abort"` profile. Same
//! panic strategy, same eventual runtime behavior (both abort), different
//! verdict — because the boundary tracks MIR shape, not panic semantics.

#[no_alloc_check::no_alloc]
fn root(values: &[u32], index: usize) -> u32 {
    *values.get(index).unwrap()
}

fn main() {
    println!("{}", root(&[1], 0));
}
