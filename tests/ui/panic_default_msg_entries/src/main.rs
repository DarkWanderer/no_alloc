//! The argument-less `panic!()`, `unreachable!()`, `todo!()` and
//! `unimplemented!()` expand to `core::panicking::panic_default_msg`,
//! `unreachable_default_msg`, `todo_default_msg` and
//! `unimplemented_default_msg`. Those wrappers have no exported MIR, so each
//! root passes only because the panic table (ADR 0008) stops the walk at them;
//! without the entries each would reject with "no statically available MIR
//! body".

#[no_alloc_check::no_alloc]
fn bare_panic(flag: bool) -> u32 {
    if flag {
        panic!();
    }
    1
}

#[no_alloc_check::no_alloc]
fn bare_unreachable(flag: bool) -> u32 {
    if flag {
        unreachable!();
    }
    2
}

#[no_alloc_check::no_alloc]
fn bare_todo(flag: bool) -> u32 {
    if flag {
        todo!();
    }
    3
}

#[no_alloc_check::no_alloc]
fn bare_unimplemented(flag: bool) -> u32 {
    if flag {
        unimplemented!();
    }
    4
}

fn main() {
    let flag = std::env::args().count() > 99;
    println!(
        "{} {} {} {}",
        bare_panic(flag),
        bare_unreachable(flag),
        bare_todo(flag),
        bare_unimplemented(flag)
    );
}
