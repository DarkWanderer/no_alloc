//! `--strict` does not consult the panic table (ADR 0008), and this is where
//! that is observable without the result being a pass. A build script is a
//! host unit: Cargo compiles it under `unwind` against the precompiled host
//! sysroot even when the target is rebuilt with `-Cpanic=immediate-abort`
//! (see `strict_host_root`). So in strict mode these two roots reject
//! exactly as the default mode used to: `unwrap` at
//! `core::option::unwrap_failed`, which the default mode would stop at as a
//! panic entry point, and indexing at its `Assert`, which `--strict` still
//! rejects under `unwind` (ADR 0003).
//!
//! It is also where the intrinsic-fallback walk (ADR 0005) stays observable.
//! The only thing in `ptr::swap_nonoverlapping`, the fallback body of
//! `typed_swap_nonoverlapping`, that the traversal can trip on is its
//! UB-check panic path. In the default mode that path is out of scope, and
//! under `--strict` on the target it ends in `abort`, so `intrinsic_fallback`
//! passes whether or not the fallback is walked. Here, on the host, it still
//! rejects: `host_swap_large` (optimized, so codegen calls the fallback) is
//! rejected inside `swap_nonoverlapping`, while `host_swap_scalar` (an
//! SSA-standalone type, swapped in place) passes.

#[no_alloc_check::no_alloc]
fn host_unwrap(values: &[u32], index: usize) -> u32 {
    *values.get(index).unwrap()
}

#[no_alloc_check::no_alloc]
fn host_index(values: &[u32], index: usize) -> u32 {
    values[index]
}

#[no_alloc_check::no_alloc]
fn host_swap_large(a: &mut [u64; 16], b: &mut [u64; 16]) {
    std::mem::swap(a, b);
}

#[no_alloc_check::no_alloc]
fn host_swap_scalar(a: &mut u64, b: &mut u64) {
    std::mem::swap(a, b);
}

fn main() {
    let (mut a, mut b) = ([1; 16], [2; 16]);
    host_swap_large(&mut a, &mut b);
    let (mut c, mut d) = (3, 4);
    host_swap_scalar(&mut c, &mut d);
    let values = [1, 2];
    println!(
        "cargo::rerun-if-changed=build.rs {} {}",
        host_unwrap(&values, 0),
        host_index(&values, 1)
    );
}
