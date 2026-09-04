//! Regression fixture for F1 (soundness review): a closure that only
//! natively implements `Fn`/`FnMut` (it doesn't consume its captures) gets
//! its `FnOnce::call_once` compiler-synthesized as `ShimKind::ClosureOnce`,
//! which just borrows `self` and makes a real, resolvable call to the same
//! closure's own `call_mut`.
//!
//! Before F1, that shim was rejected outright: the old gate checked
//! `is_mir_available` on `FnOnce::call_once`'s own bodiless trait
//! declaration, not on anything about this closure. That was a false
//! positive — this closure never touches the allocator. After F1 the shim
//! is traversed like any other body, reaching the closure's real code and
//! correctly passing.

fn call_once<F: FnOnce() -> i32>(f: F) -> i32 {
    f()
}

#[no_alloc_check::no_alloc]
fn root(x: i32) -> i32 {
    let f = move || x.wrapping_add(1);
    call_once(f)
}

fn main() {
    println!("{}", root(41));
}
