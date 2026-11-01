//! `Ord::min`/`Ord::max` for integers lower to the `integer_min` and
//! `integer_max` intrinsics, which the backend turns into a direct
//! `llvm.umin`/`smin`/`umax`/`smax` call. They are in the non-allocating
//! intrinsic table (ADR 0005), so a root that only compares integers passes
//! in the default mode. The inherent-looking `u32::min` and `i64::max` are
//! the same `Ord` methods, called through the trait.

#[no_alloc_check::no_alloc]
fn min_unsigned(a: u32, b: u32) -> u32 {
    Ord::min(a, b)
}

#[no_alloc_check::no_alloc]
fn max_signed(a: i64, b: i64) -> i64 {
    Ord::max(a, b)
}

#[no_alloc_check::no_alloc]
fn method_forms(a: u32, b: i64) -> i64 {
    i64::from(a.min(7)) + b.max(-3)
}

fn main() {
    println!("{} {} {}", min_unsigned(1, 2), max_signed(1, 2), method_forms(5, 6));
}
