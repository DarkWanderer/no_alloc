//! Every iterator pattern in `docs/iterators.md`, as one runnable crate.
//!
//! ```bash
//! cargo no-alloc --immediate-abort -- build
//! ```
//!
//! Expect 33 of the 35 roots to pass, `sort_in_place` to be rejected at a
//! genuine `fn` pointer inside the sort, and `collects_into_vec` to be
//! reported as a real violation. Drop the `--immediate-abort` and all 33 of
//! those passes turn into rejections inside the panic machinery — that is
//! the whole point of the mode, and of the document.

use no_alloc_check::no_alloc;

// ---------------------------------------------------------------------
// Driving a slice iterator
// ---------------------------------------------------------------------

#[no_alloc]
fn for_loop(buf: &[f32]) -> f32 {
    let mut acc = 0.0;
    for &x in buf {
        acc += x;
    }
    acc
}

#[no_alloc]
fn manual_next(buf: &[f32]) -> f32 {
    let mut it = buf.iter();
    match it.next() {
        Some(x) => *x,
        None => 0.0,
    }
}

#[no_alloc]
fn iter_mut_scale(buf: &mut [f32]) {
    for x in buf.iter_mut() {
        *x *= 2.0;
    }
}

#[no_alloc]
fn range_sum(n: u32) -> u32 {
    (0..n).map(|x| x.wrapping_mul(2)).sum()
}

#[no_alloc]
fn step_by_sum(n: u32) -> u32 {
    (0..n).step_by(2).fold(0u32, |a, b| a.wrapping_add(b))
}

#[no_alloc]
fn array_into_iter(buf: [f32; 4]) -> f32 {
    buf.into_iter().fold(0.0, |a, b| a + b)
}

// ---------------------------------------------------------------------
// Consuming: fold, sum, count, and the short-circuiting searches
// ---------------------------------------------------------------------

#[no_alloc]
fn iter_sum(buf: &[f32]) -> f32 {
    buf.iter().sum()
}

#[no_alloc]
fn copied_fold(buf: &[f32]) -> f32 {
    buf.iter().copied().fold(0.0, |a, b| a + b)
}

#[no_alloc]
fn for_each_sum(buf: &[f32]) -> f32 {
    let mut acc = 0.0;
    buf.iter().for_each(|x| acc += x);
    acc
}

#[no_alloc]
fn count_items(buf: &[f32]) -> usize {
    buf.iter().count()
}

#[no_alloc]
fn nth_item(buf: &[f32]) -> f32 {
    match buf.iter().nth(1) {
        Some(x) => *x,
        None => 0.0,
    }
}

#[no_alloc]
fn any_above(buf: &[f32]) -> bool {
    buf.iter().any(|x| *x > 1.0)
}

#[no_alloc]
fn all_below(buf: &[f32]) -> bool {
    buf.iter().all(|x| *x < 10.0)
}

#[no_alloc]
fn find_first(buf: &[f32]) -> f32 {
    match buf.iter().find(|x| **x > 1.0) {
        Some(x) => *x,
        None => 0.0,
    }
}

#[no_alloc]
fn position_of(buf: &[f32]) -> usize {
    match buf.iter().position(|x| *x > 1.0) {
        Some(i) => i,
        None => 0,
    }
}

// ---------------------------------------------------------------------
// Adapters
// ---------------------------------------------------------------------

#[no_alloc]
fn map_sum(buf: &[f32]) -> f32 {
    buf.iter().map(|x| x * 2.0).sum()
}

#[no_alloc]
fn filter_count(buf: &[f32]) -> usize {
    buf.iter().filter(|x| **x > 0.0).count()
}

#[no_alloc]
fn enumerate_last(buf: &[f32]) -> usize {
    let mut n = 0;
    for (i, _) in buf.iter().enumerate() {
        n = i;
    }
    n
}

#[no_alloc]
fn zip_dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| x * y)
        .fold(0.0, |s, v| s + v)
}

#[no_alloc]
fn chained(a: &[f32], b: &[f32]) -> f32 {
    a.iter().chain(b.iter()).fold(0.0, |acc, x| acc + x)
}

#[no_alloc]
fn take_skip(buf: &[f32]) -> f32 {
    buf.iter().skip(1).take(2).fold(0.0, |a, b| a + b)
}

#[no_alloc]
fn take_while_sum(buf: &[f32]) -> f32 {
    buf.iter().take_while(|x| **x < 10.0).fold(0.0, |a, b| a + b)
}

#[no_alloc]
fn rev_sum(buf: &[f32]) -> f32 {
    let mut acc = 0.0;
    for &x in buf.iter().rev() {
        acc += x;
    }
    acc
}

#[no_alloc]
fn peekable_first(buf: &[f32]) -> f32 {
    let mut it = buf.iter().peekable();
    match it.peek() {
        Some(x) => **x,
        None => 0.0,
    }
}

// ---------------------------------------------------------------------
// Slice views
// ---------------------------------------------------------------------

#[no_alloc]
fn chunks_exact_sum(buf: &[f32]) -> f32 {
    let mut acc = 0.0;
    for c in buf.chunks_exact(2) {
        acc += c[0];
    }
    acc
}

#[no_alloc]
fn rchunks_sum(buf: &[f32]) -> f32 {
    let mut acc = 0.0;
    for c in buf.rchunks(2) {
        acc += c[0];
    }
    acc
}

#[no_alloc]
fn windows_sum(buf: &[f32]) -> f32 {
    let mut acc = 0.0;
    for w in buf.windows(2) {
        acc += w[0];
    }
    acc
}

#[no_alloc]
fn binary_search(buf: &[u32]) -> usize {
    match buf.binary_search(&2) {
        Ok(i) => i,
        Err(i) => i,
    }
}

// ---------------------------------------------------------------------
// Comparison-driven adapters. Each of these reaches its callback as a
// function item or a `&mut F` reborrow, through a `FnMut::call_mut` shim —
// walkable since ADR 0007, and the reason this block is no longer the
// "does not pass" section. `sort_in_place` is the one that still rejects,
// on a real `fn` pointer inside the sort's partition selection.
// ---------------------------------------------------------------------

#[no_alloc]
fn max_plain(buf: &[u32]) -> u32 {
    match buf.iter().max() {
        Some(x) => *x,
        None => 0,
    }
}

#[no_alloc]
fn max_by_closure(buf: &[u32]) -> u32 {
    match buf.iter().max_by(|a, b| a.cmp(b)) {
        Some(x) => *x,
        None => 0,
    }
}

#[no_alloc]
fn min_by_key(buf: &[u32]) -> u32 {
    match buf.iter().min_by_key(|x| **x) {
        Some(x) => *x,
        None => 0,
    }
}

#[no_alloc]
fn flat_map_count(buf: &[u32]) -> usize {
    buf.iter().flat_map(|x| core::iter::repeat_n(*x, 2)).count()
}

#[no_alloc]
fn scan_last(buf: &[u32]) -> u32 {
    buf.iter()
        .scan(0u32, |s, x| {
            *s = s.wrapping_add(*x);
            Some(*s)
        })
        .last()
        .unwrap_or(0)
}

#[no_alloc]
fn sort_in_place(buf: &mut [u32]) {
    buf.sort_unstable();
}

// ---------------------------------------------------------------------
// The control: an adapter that really does allocate.
// ---------------------------------------------------------------------

#[no_alloc]
fn collects_into_vec(buf: &[u32]) -> usize {
    let collected: Vec<u32> = buf.iter().copied().collect();
    collected.len()
}

fn main() {
    let mut f = [1.0f32, 2.0, 3.0, 4.0];
    let mut u = [3u32, 1, 2];
    println!(
        "{} {} {} {} {} {} {} {} {} {} {} {} {}",
        for_loop(&f),
        manual_next(&f),
        range_sum(4),
        step_by_sum(6),
        array_into_iter(f),
        iter_sum(&f),
        copied_fold(&f),
        for_each_sum(&f),
        count_items(&f),
        nth_item(&f),
        any_above(&f),
        all_below(&f),
        find_first(&f),
    );
    println!(
        "{} {} {} {} {} {} {} {} {} {}",
        position_of(&f),
        map_sum(&f),
        filter_count(&f),
        enumerate_last(&f),
        zip_dot(&f, &f),
        chained(&f, &f),
        take_skip(&f),
        take_while_sum(&f),
        rev_sum(&f),
        peekable_first(&f),
    );
    println!(
        "{} {} {} {} {} {} {} {} {} {}",
        chunks_exact_sum(&f),
        rchunks_sum(&f),
        windows_sum(&f),
        binary_search(&u),
        max_plain(&u),
        max_by_closure(&u),
        min_by_key(&u),
        flat_map_count(&u),
        scan_last(&u),
        collects_into_vec(&u),
    );
    iter_mut_scale(&mut f);
    sort_in_place(&mut u);
    println!("{f:?} {u:?}");
}
