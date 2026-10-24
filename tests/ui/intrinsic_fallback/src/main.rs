//! An intrinsic in the non-allocating table is only vouched for when the
//! backend lowers it itself. `mem::swap` is `typed_swap_nonoverlapping`,
//! which `rustc_codegen_ssa` swaps in place for an SSA-standalone type (or
//! at `opt-level = 0`); for a large array when optimizing, it emits a call
//! to the intrinsic's Rust fallback body (`ptr::swap_nonoverlapping`)
//! instead. That body is walked, so its UB-check panic path is found and
//! rejected rather than waved through with the intrinsic.

#[no_alloc_check::no_alloc]
fn swap_large(a: &mut [u64; 16], b: &mut [u64; 16]) {
    std::mem::swap(a, b);
}

#[no_alloc_check::no_alloc]
fn swap_scalar(a: &mut u64, b: &mut u64) {
    std::mem::swap(a, b);
}

fn main() {
    let mut a = [1; 16];
    let mut b = [2; 16];
    swap_large(&mut a, &mut b);
    let (mut c, mut d) = (3, 4);
    swap_scalar(&mut c, &mut d);
    println!("{}", a[0] + b[0] + c + d);
}
