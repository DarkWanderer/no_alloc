#![allow(unexpected_cfgs)] // real usage sets this via [lints.rust] in Cargo.toml; see README

#[no_alloc_macros::no_alloc]
fn compute(x: i32) -> i32 {
    x + 1
}

fn main() {
    assert_eq!(compute(41), 42);
}
