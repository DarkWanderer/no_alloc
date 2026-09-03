//! Regression fixture for F5 (soundness review): every fixture that
//! allocates so far goes through `__rust_alloc_zeroed` (`Box::new`,
//! `Vec::with_capacity`, ...) or `__rust_dealloc`. Plain `__rust_alloc`
//! (the `ALLOCATOR` flag in `leaf.rs`) had no fixture at all, despite
//! `README.md` naming all four flags. `std::alloc::alloc` is the direct,
//! uninitialized entry point that hits it.

use std::alloc::{alloc, Layout};

#[no_alloc_check::no_alloc]
fn root() -> *mut u8 {
    unsafe { alloc(Layout::new::<[u8; 32]>()) }
}

fn main() {
    println!("{:?}", root());
}
