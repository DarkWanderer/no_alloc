pub trait Work {
    fn work() -> usize;
}

/// Never instantiated by `app` — no caller anywhere in this build ever
/// substitutes a concrete `T`, so `collect_and_partition_mono_items` never
/// produces an instance for it. `roots.rs` still discovers its *definition*
/// (via the `#[no_alloc]` marker, seeded from `tcx.iter_local_def_id`), so
/// it must be reported as `NotInstantiated` rather than silently dropped.
#[no_alloc_check::no_alloc]
pub fn uncalled_root<T: Work>() -> usize {
    T::work()
}
