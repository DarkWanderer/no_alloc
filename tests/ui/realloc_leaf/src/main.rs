//! Regression fixture for F5 (soundness review): the `REALLOCATOR` flag in
//! `leaf.rs` had no fixture. `Vec::push` growth doesn't isolate it cleanly —
//! `RawVecInner::finish_grow` branches at runtime between
//! `Allocator::grow` and `Allocator::allocate`, and this traversal (not
//! path-sensitive) finds whichever branch's block comes first in MIR,
//! which in practice was `allocate`. `std::alloc::realloc` is the direct
//! entry point that hits `__rust_realloc` with no such branch.

use std::alloc::{alloc, dealloc, realloc, Layout};

#[no_alloc_check::no_alloc]
fn root(ptr: *mut u8) -> *mut u8 {
    unsafe { realloc(ptr, Layout::new::<[u8; 32]>(), 64) }
}

fn main() {
    let ptr = unsafe { alloc(Layout::new::<[u8; 32]>()) };
    let ptr = root(ptr);
    unsafe { dealloc(ptr, Layout::new::<[u8; 64]>()) };
}
