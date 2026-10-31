// The raw marker, not `#[no_alloc_check::no_alloc]`: the proc macro refuses
// `async fn`, but a dependency can still carry the attribute directly, and the
// driver's `register_tool` crate attributes make it legal in every crate.
#[cfg_attr(no_alloc_check, no_alloc_tool::root)]
pub async fn work<T: Default>() -> T {
    *Box::new(T::default())
}

// Instantiates `work::<u64>` in this crate so the app links to that copy under
// share-generics and the instance is absent from the app's mono items.
pub fn helper() {
    drop(work::<u64>());
}
