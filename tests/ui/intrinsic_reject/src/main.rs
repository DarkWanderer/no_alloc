//! The other half of the intrinsic table (ADR 0005): it is an allowlist, and
//! `catch_unwind` is deliberately not on it. The intrinsic runs the function
//! pointers it is handed, so what it reaches is exactly what the traversal
//! cannot see — the same reason a `fnptr` call rejects.
#![feature(core_intrinsics)]
#![allow(internal_features)]

unsafe fn attempt(_data: *mut u8) {}

unsafe fn recover(_data: *mut u8, _payload: *mut u8) {}

#[no_alloc_check::no_alloc]
fn root(data: *mut u8) -> bool {
    unsafe { std::intrinsics::catch_unwind(attempt, data, recover) }
}

fn main() {
    println!("{}", root(std::ptr::null_mut()));
}
