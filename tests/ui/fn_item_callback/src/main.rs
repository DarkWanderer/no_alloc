//! A callback that arrives as a function *item* rather than a closure, which
//! is how `Iterator::max` reaches `Ord::cmp` and how most of the standard
//! library's comparison adapters are written.
//!
//! Two things have to work for this to be checkable, both ADR 0007: the call
//! goes through a compiler-generated `FnMut::call_mut` shim, whose body is
//! built on demand rather than found by `is_mir_available`; and inside that
//! shim the callee arrives as a moved `FnDef` local, not as a literal path,
//! so it has to be resolved from the operand's type. Neither is a fn
//! pointer — the type still names exactly one function.

fn double(x: u32) -> u32 {
    x.wrapping_mul(2)
}

fn apply<F: FnMut(u32) -> u32>(mut f: F, x: u32) -> u32 {
    f(x)
}

fn apply_by_ref<F: FnMut(u32) -> u32>(f: &mut F, x: u32) -> u32 {
    f(x)
}

#[no_alloc_check::no_alloc]
fn root(x: u32) -> u32 {
    let mut callback = double;
    apply(double, x).wrapping_add(apply_by_ref(&mut callback, x))
}

fn main() {
    println!("{}", root(21));
}
