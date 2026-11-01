//! Originally the regression fixture for F2 (soundness review): under the
//! old default, `.unwrap()` lowered to a `Call` into `unwrap_failed`, which
//! has no MIR in the precompiled sysroot, so it rejected even under
//! `panic = "abort"`, where `panic_abort`'s `Assert` passed. ADR 0008 moved
//! that boundary: in the default mode `core::option::unwrap_failed` is a
//! panic entry point, so the explicit panic call and the implicit `Assert`
//! now get the same verdict under the same profile. This case pins the
//! `panic = "abort"` half of that; `panic_out_of_scope` pins `unwind`, and
//! `strict_host_panic_reject` pins that `--strict` still walks into
//! `unwrap_failed` and rejects there.

#[no_alloc_check::no_alloc]
fn root(values: &[u32], index: usize) -> u32 {
    *values.get(index).unwrap()
}

fn main() {
    println!("{}", root(&[1], 0));
}
