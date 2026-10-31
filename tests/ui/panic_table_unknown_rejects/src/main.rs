//! The panic table is an allowlist (ADR 0008). Each root here ends in a
//! sysroot function that diverges (`-> !`) without being a panic, so none
//! of them is in the table, and the default mode handles each like any
//! other callee: walked if it has MIR, rejected if it does not. Treating
//! "diverges" as "panics" would let a path that allocates and then aborts
//! or exits pass, and `handle_alloc_error` is what an allocation failure
//! calls, so excusing it would excuse the allocator.

use std::alloc::Layout;

#[no_alloc_check::no_alloc]
fn aborts() -> ! {
    std::process::abort()
}

#[no_alloc_check::no_alloc]
fn exits(code: i32) -> ! {
    std::process::exit(code)
}

#[no_alloc_check::no_alloc]
fn alloc_error(layout: Layout) -> ! {
    std::alloc::handle_alloc_error(layout)
}

fn main() {
    if std::env::args().count() > 3 {
        alloc_error(Layout::new::<u64>());
    }
    if std::env::args().count() > 2 {
        exits(1);
    }
    if std::env::args().count() > 1 {
        aborts();
    }
}
