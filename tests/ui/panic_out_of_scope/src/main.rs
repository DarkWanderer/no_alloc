//! The default checking mode (ADR 0008) under Rust's default
//! `panic = "unwind"`, with no `--strict` and no rebuilt sysroot. Every root
//! here can panic, and none allocates on a path that does not: each passes
//! because its panic paths end at a sysroot panic entry point in the panic
//! table, or at an `Assert` terminator, both of which are out of scope.
//!
//! Under the previous default every one of these rejected, at a panic
//! function the precompiled sysroot has no MIR for (or, for `Assert`, at
//! the terminator itself under `unwind`). The table is matched on the
//! *defining* path: `unwraps` reaches `core::option::unwrap_failed`, which
//! `def_path_str` prints as `std::option::unwrap_failed` under the `std`
//! prelude, so this case is also the proof that the two spellings agree.

use std::cell::RefCell;

#[no_alloc_check::no_alloc]
fn unwraps(values: &[u32], index: usize) -> u32 {
    *values.get(index).unwrap()
}

#[no_alloc_check::no_alloc]
fn expects_option(value: Option<u32>) -> u32 {
    value.expect("present")
}

#[no_alloc_check::no_alloc]
fn expects_result(value: Result<u32, u8>) -> u32 {
    value.expect("present")
}

/// A bounds-checked index lowers to an `Assert`, not a call.
#[no_alloc_check::no_alloc]
fn indexes(values: &[u32], index: usize) -> u32 {
    values[index]
}

/// Range indexing goes through `SliceIndex` to `slice_index_fail`.
#[no_alloc_check::no_alloc]
fn slices(values: &[u32], end: usize) -> u32 {
    values[..end].len() as u32
}

/// Overflow-checked arithmetic (debug profile) is another `Assert`.
#[no_alloc_check::no_alloc]
fn adds(a: u32, b: u32) -> u32 {
    a + b
}

#[no_alloc_check::no_alloc]
fn panics_plainly(flag: bool) -> u32 {
    if flag {
        panic!();
    }
    1
}

/// A formatted message: `format_args!` builds an `Arguments` on the stack,
/// and `panic_fmt` is the entry point.
#[no_alloc_check::no_alloc]
fn panics_with_message(value: u32) -> u32 {
    if value > 10 {
        panic!("value {value} is too large");
    }
    value
}

#[no_alloc_check::no_alloc]
fn borrows(cell: &RefCell<u32>) -> u32 {
    *cell.borrow_mut() += 1;
    *cell.borrow()
}

#[no_alloc_check::no_alloc]
fn iterates(values: &[u32]) -> u32 {
    values.iter().map(|v| v * 2).sum()
}

fn main() {
    let values = [1, 2, 3];
    let cell = RefCell::new(1);
    println!(
        "{} {} {} {} {} {} {} {} {} {}",
        unwraps(&values, 0),
        expects_option(Some(1)),
        expects_result(Ok(1)),
        indexes(&values, 1),
        slices(&values, 2),
        adds(1, 2),
        panics_plainly(false),
        panics_with_message(3),
        borrows(&cell),
        iterates(&values),
    );
}
