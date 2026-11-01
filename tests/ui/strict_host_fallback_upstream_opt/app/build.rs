//! ADR 0005's per-caller rule, observed where a panic path is still in scope
//! and still unresolvable: a host unit under `--strict`, which compiles
//! under `unwind` against the precompiled host sysroot (see
//! `strict_host_panic_reject`). In the default mode, and on the target under
//! `--strict`, the fallback body's only rejecting edge (its UB-check panic)
//! is out of scope or ends in `abort`, so `intrinsic_fallback_upstream_opt`
//! passes whether the fallback was walked or not.
//!
//! The build script is opt-level 0; its build-dependency is opt-level 1 and
//! exports `swap_generic::<[u64; 16]>`, compiled with `mem::swap` through
//! the intrinsic's fallback body.

// Reuses the dependency's instantiation, so the opt-level-0 in-place
// shortcut must not apply: rejected inside `ptr::swap_nonoverlapping`.
#[no_alloc_check::no_alloc]
fn shared_swap(a: &mut [u64; 16], b: &mut [u64; 16]) {
    upstream_opt_dep::swap_generic(a, b)
}

// Never called, and `dep` does not instantiate `swap_generic::<[u32; 16]>`,
// so this swap is local to the opt-level-0 build script and lowered in
// place: passes.
#[allow(dead_code)]
#[no_alloc_check::no_alloc]
fn dead_local(a: &mut [u32; 16], b: &mut [u32; 16]) {
    upstream_opt_dep::swap_generic(a, b)
}

fn main() {
    let (mut a, mut b) = ([1; 16], [2; 16]);
    upstream_opt_dep::swap_big(&mut a, &mut b);
    shared_swap(&mut a, &mut b);
    println!("cargo::rerun-if-changed=build.rs");
}
