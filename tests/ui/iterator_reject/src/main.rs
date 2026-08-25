//! The default configuration's answer for iterators, and the contrast case
//! for `iterator_immediate_abort`: `panic = "abort"` alone is not enough.
//!
//! `Iter::next` calls `usize::unchecked_sub`, whose UB-check precondition
//! calls `core::panicking::panic_nounwind_fmt` — a plain call, not an
//! `Assert` terminator, so the panic-strategy carve-out does not apply to
//! it. The precompiled sysroot ships no MIR for that function, so the edge
//! is unresolved and rejects (ADR 0003). Rebuilding std is what turns this
//! into a checkable path; see docs/iterators.md.

#[no_alloc_check::no_alloc]
fn root(buf: &[f32]) -> f32 {
    let mut acc = 0.0;
    for &x in buf {
        acc += x;
    }
    acc
}

fn main() {
    println!("{}", root(&[1.0, 2.0, 3.0]));
}
