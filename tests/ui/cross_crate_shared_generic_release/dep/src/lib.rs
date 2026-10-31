#[no_alloc_check::no_alloc]
pub fn process<T: Default>() -> usize {
    let v: Vec<T> = vec![T::default()];
    v.len()
}

// `#[inline(never)]` makes `outer::<u32>` shared even at opt-level 3, so the
// app never walks its body; the marked callee inside it is only found by
// following an upstream-shared body.
#[inline(never)]
pub fn outer<T: Default>() -> usize {
    process::<T>()
}

pub fn helper() -> usize {
    outer::<u32>()
}
