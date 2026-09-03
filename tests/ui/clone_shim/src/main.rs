//! Regression fixture for F1 (soundness review): a tuple has no
//! library-source `Clone` impl (unlike arrays, which do on this toolchain),
//! so `<(i32, String) as Clone>::clone` resolves to the compiler-generated
//! `ShimKind::Clone` shim. Before F1, that shim's *own* def_id
//! (`Clone::clone`'s bodiless trait declaration) made the old MIR-
//! availability gate reject it immediately, without ever looking inside.
//! After F1 the shim is traversed like any other body, reaching the real
//! per-field `<String as Clone>::clone` call — a deeper, more precise
//! chain, still `rejected` either way (this repo's traversal does not
//! resolve fully into `String::clone`'s own internals, for reasons
//! unrelated to this fixture).

fn clone_it<T: Clone>(value: &T) -> T {
    value.clone()
}

#[no_alloc_check::no_alloc]
fn root(pair: &(i32, String)) -> (i32, String) {
    clone_it(pair)
}

fn main() {
    let pair = (1, String::from("hi"));
    println!("{:?}", root(&pair));
}
