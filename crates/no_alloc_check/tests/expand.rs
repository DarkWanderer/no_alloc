//! Macro-expansion tests, isolated from the rustc-internals driver: does
//! the attribute expand cleanly on a normal build (the zero-footprint
//! claim, at the macro-crate level rather than the full driver-integration
//! level covered by `tests/ui/` at the repo root), and does it reject
//! arguments and unsupported item shapes as documented.

#[test]
fn expand() {
    let t = trybuild::TestCases::new();
    t.pass("tests/expand/pass/*.rs");
    t.compile_fail("tests/expand/fail/*.rs");
}
