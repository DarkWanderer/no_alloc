// KNOWN FALSE REJECTION, pinned so a change is noticed, not endorsed as
// desirable. With share-generics on at opt-level 0, `dep` (also opt-level 0)
// exports `mem::swap::<[u64; 16]>`, so `should_codegen_locally` reports the
// instance as upstream-shared and the checker walks the fallback body even
// though `dep` lowered the swap in place. Conservative under ADR 0003.
//
// The re-export is what links `dep` into the artifact; without it no upstream
// instantiation is linked and this root passes.
pub use shared_swap_dep::swap_big;

#[no_alloc_check::no_alloc]
pub fn local_swap(a: &mut [u64; 16], b: &mut [u64; 16]) {
    core::mem::swap(a, b)
}
