// The checker injects this same feature through `-Zcrate-attr`; a crate that
// already enables it must still be built and checked, not fail on the
// deny-by-default `duplicate_features` lint.
#![feature(rustc_attrs)]
#![allow(internal_features)]

#[no_alloc_check::no_alloc]
#[inline(never)]
fn root(x: u8) -> u8 {
    *Box::new(x)
}

fn main() {
    println!("{}", root(1));
}
