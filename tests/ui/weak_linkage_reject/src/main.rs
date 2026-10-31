#![feature(linkage)]

// A weak definition: the linker is free to bind `hook` to a strong definition
// in another object, so this body's MIR says nothing about what runs.
#[unsafe(no_mangle)]
#[linkage = "weak"]
pub extern "C" fn hook() -> usize {
    0
}

#[no_alloc_check::no_alloc]
fn root() -> usize {
    hook()
}

fn main() {
    println!("{}", root());
}
