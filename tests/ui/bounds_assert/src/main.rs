//! An `Assert` under Rust's default `panic = "unwind"`. It used to reject
//! (`AssertUnderUnwind`); in the default mode a failing assertion is a
//! panic, which is out of scope under every strategy (ADR 0008).
//! `strict_host_panic_reject` pins the strict-mode rejection.

#[no_alloc_check::no_alloc]
fn root(values: &[u32], index: usize) -> u32 {
    values[index]
}

fn main() {
    println!("{}", root(&[1], 0));
}
