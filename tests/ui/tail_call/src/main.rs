//! Regression fixture for F5 (soundness review): `TerminatorKind::TailCall`
//! (the `become` keyword, `#![feature(explicit_tail_calls)]`) is named in
//! both the README's guarantee and ADR 0003's terminator table, but had no
//! fixture exercising it at all.

#![feature(explicit_tail_calls)]

fn leaf() -> i32 {
    let b = Box::new(9);
    *b
}

fn helper() -> i32 {
    become leaf()
}

#[no_alloc_check::no_alloc]
fn root() -> i32 {
    become helper()
}

fn main() {
    println!("{}", root());
}
