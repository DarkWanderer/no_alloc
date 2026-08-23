#![allow(unexpected_cfgs)] // real usage sets this via [lints.rust] in Cargo.toml; see README

#[no_alloc_check::no_alloc]
fn identity<T>(x: T) -> T {
    x
}

fn main() {
    assert_eq!(identity(42), 42);
}
