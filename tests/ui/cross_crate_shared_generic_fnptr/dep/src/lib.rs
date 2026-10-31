#[no_alloc_check::no_alloc]
#[inline(never)]
pub fn process<T: Default>() -> usize {
    let v: Vec<T> = vec![T::default()];
    v.len()
}

// Instantiates `process::<u32>` in this crate, so with share-generics the
// downstream crate links to that copy instead of codegening its own and the
// instance never shows up among the app's mono items.
pub fn helper() -> usize {
    process::<u32>()
}
