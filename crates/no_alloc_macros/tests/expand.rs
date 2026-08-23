//! Macro-expansion tests, isolated from the rustc-internals driver: does
//! the attribute expand cleanly on a normal build (the zero-footprint
//! claim, at the macro-crate level rather than the full driver-integration
//! level covered by `tests/ui/` at the repo root), and does it reject
//! arguments the way `no_alloc_macros::no_alloc`'s doc comment says it does.

#[test]
fn expand() {
    let t = trybuild::TestCases::new();
    t.pass("tests/expand/pass/*.rs");
    t.compile_fail("tests/expand/fail/*.rs");
}
