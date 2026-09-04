//! Regression fixture for F3 (soundness review): reading a same-crate
//! `#[thread_local]` static lowers to a bare `Rvalue::ThreadLocalRef`
//! *statement*, not a `Call` terminator — this traversal previously
//! inspected only terminators, so this was invisible to it and silently
//! passed. rustc's own doc comment on `Rvalue::ThreadLocalRef` says this
//! "is a runtime operation that actually executes code and is in this
//! sense more like a function call" (it can lower to `__tls_get_addr`,
//! which glibc can route through `malloc` on first access to a `dlopen`ed
//! module). No MIR callee exists to follow, so — consistent with
//! `ShimKind::ThreadLocal`'s cross-crate equivalent — it is rejected
//! outright rather than assumed benign.

#![feature(thread_local)]

#[thread_local]
static COUNTER: i32 = 7;

#[no_alloc_check::no_alloc]
fn root() -> i32 {
    COUNTER
}

fn main() {
    println!("{}", root());
}
