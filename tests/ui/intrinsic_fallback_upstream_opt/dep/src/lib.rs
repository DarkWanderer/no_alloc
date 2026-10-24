// A non-`#[inline]` generic: `mem::swap` itself is `#[inline]`, which an
// optimizing crate emits as a local copy and never exports, so it would not be
// shared. This wrapper is what dep exports as `swap_generic::<[u64; 16]>`.
pub fn swap_generic<T>(a: &mut T, b: &mut T) {
    core::mem::swap(a, b)
}

// Instantiates the generic in `dep` for `[u64; 16]`.
pub fn swap_big(a: &mut [u64; 16], b: &mut [u64; 16]) {
    swap_generic(a, b)
}
