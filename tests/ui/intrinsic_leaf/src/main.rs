//! An intrinsic is a resolved callee whose body lives in the compiler, not
//! a call the traversal failed to resolve: the ones named in the
//! non-allocating intrinsic table are checked, not rejected (ADR 0005).
//! `f32::sqrt` is `intrinsics::sqrtf32`, and the bit ops below are
//! `ctpop`/`ctlz`/`rotate_left` — none of which survive as a MIR body to
//! walk into.

#[no_alloc_check::no_alloc]
fn float_root(x: f32) -> f32 {
    x.sqrt()
}

#[no_alloc_check::no_alloc]
fn bits_root(x: u32) -> u32 {
    x.count_ones()
        .wrapping_add(x.leading_zeros())
        .wrapping_add(x.rotate_left(3))
}

fn main() {
    println!("{} {}", float_root(2.0), bits_root(7));
}
