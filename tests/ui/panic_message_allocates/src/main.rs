//! Only what happens *inside* a panic entry point is out of scope in the
//! default mode (ADR 0008). The message here is built in the root's own
//! body, before `panic_fmt` is called, and building it allocates — so the
//! root is a violation even though the allocation only happens on a path
//! that goes on to panic. The traversal is not path-sensitive, and the
//! exclusion starts at the call, not at the branch leading to it.

#[no_alloc_check::no_alloc]
fn root(value: u32) -> u32 {
    if value > 10 {
        let message = String::from("too large");
        panic!("{message}");
    }
    value
}

fn main() {
    println!("{}", root(3));
}
