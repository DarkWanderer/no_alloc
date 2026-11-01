//! The panic table is matched on a sysroot function's defining path, and a
//! crate only counts as `core`/`alloc`/`std` if it defines that library's
//! anchor lang item (ADR 0008). So user code cannot borrow an entry's name to
//! hide an allocation: these two functions mimic `core::panicking::panic_fmt`
//! and `core::option::unwrap_failed` by path and by name, both allocate
//! before diverging, and both roots are violations in the default mode.
//! The first is the reason the key is not `def_path_str`: for this local
//! item it prints exactly `core::panicking::panic_fmt`.

mod core {
    pub mod panicking {
        pub fn panic_fmt(code: u32) -> ! {
            let boxed = Box::new(code);
            std::process::exit(*boxed as i32)
        }
    }
}

fn unwrap_failed(code: u32) -> ! {
    let boxed = Box::new(code);
    std::process::exit(*boxed as i32)
}

#[no_alloc_check::no_alloc]
fn via_module_path(flag: bool) -> u32 {
    if flag {
        crate::core::panicking::panic_fmt(1);
    }
    0
}

#[no_alloc_check::no_alloc]
fn via_function_name(flag: bool) -> u32 {
    if flag {
        unwrap_failed(2);
    }
    0
}

fn main() {
    let flag = std::env::args().count() > 1;
    println!("{} {}", via_module_path(flag), via_function_name(flag));
}
