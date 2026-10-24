//! Under `--immediate-abort` an `Assert` terminator is not a carve-out
//! (ADR 0006): a failing assertion lowers to a call to a core panic lang
//! item (`panic_bounds_check`, `panic_const_add_overflow`, ...), and that
//! call is resolved and walked like any other. Each root passes because
//! the lang item it reaches is `abort` in this mode, not because the
//! assertion was skipped.

#[no_alloc_check::no_alloc]
fn index(values: &[u32], index: usize) -> u32 {
    values[index]
}

#[no_alloc_check::no_alloc]
fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[no_alloc_check::no_alloc]
fn divide(a: i32, b: i32) -> i32 {
    a / b
}

fn main() {
    println!("{} {} {}", index(&[1], 0), add(1, 2), divide(6, 3));
}
