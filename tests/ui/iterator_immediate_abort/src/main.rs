//! The iterator subset (ADR 0005, docs/iterators.md). Run with
//! `--immediate-abort`, which rebuilds std with `-Cpanic=immediate-abort`:
//! every panic path then lowers to `intrinsics::abort`, which the
//! non-allocating intrinsic table classifies instead of rejecting, and the
//! slice-iterator internals become walkable all the way down.
//!
//! Note what this manifest does *not* contain: no `[profile.dev] panic`, no
//! `cargo-features`. The checker supplies the panic strategy, so the crate
//! stays buildable on stable (ADR 0002).

#[no_alloc_check::no_alloc]
fn sum_loop(buf: &[f32]) -> f32 {
    let mut acc = 0.0;
    for &x in buf {
        acc += x;
    }
    acc
}

#[no_alloc_check::no_alloc]
fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| x * y)
        .fold(0.0, |acc, v| acc + v)
}

#[no_alloc_check::no_alloc]
fn windowed(buf: &[f32]) -> f32 {
    let mut acc = 0.0;
    for w in buf.windows(2) {
        acc += w[1] - w[0];
    }
    acc
}

/// Worth pinning as a *pass*: `max` reduces through `max_by(Ord::cmp)`, so
/// the comparator arrives as a function item behind a `&mut F` reborrow and
/// the call goes through a compiler-generated `FnMut::call_mut` shim. Both
/// halves of that — walking a shim instance, and resolving a callee whose
/// operand is a moved `FnDef` rather than a literal path — are ADR 0007.
#[no_alloc_check::no_alloc]
fn largest(buf: &[u32]) -> u32 {
    match buf.iter().max() {
        Some(x) => *x,
        None => 0,
    }
}

/// The limitation that is left, and it is a real one rather than a
/// modelling gap: `sort_unstable` picks its partition implementation
/// through an actual `fn` pointer, so there is no single body to walk to.
#[no_alloc_check::no_alloc]
fn sorted(buf: &mut [u32]) {
    buf.sort_unstable();
}

fn main() {
    let d = [1.0f32, 2.0, 3.0];
    let mut u = [3u32, 1, 2];
    println!(
        "{} {} {} {}",
        sum_loop(&d),
        dot(&d, &d),
        windowed(&d),
        largest(&u)
    );
    sorted(&mut u);
    println!("{u:?}");
}
