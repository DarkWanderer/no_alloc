//! Regression fixture for F5 (soundness review): every existing cross-crate
//! fixture (`cross_crate_generic`, `multi_crate`) either calls a *generic*
//! dependency function or never actually calls across the crate boundary at
//! all. A plain (non-generic) call from a root into an ordinary dependency
//! function — the most common real-world shape — had no fixture, even
//! though it exercises a distinct path: `try_resolve` on a concrete,
//! non-generic cross-crate `DefId` rather than one substituted at the call
//! site.

#[no_alloc_check::no_alloc]
fn root_allocates() -> usize {
    call_dep::allocates()
}

#[no_alloc_check::no_alloc]
fn root_pure() -> usize {
    call_dep::pure()
}

fn main() {
    println!("{}", root_allocates());
    println!("{}", root_pure());
}
