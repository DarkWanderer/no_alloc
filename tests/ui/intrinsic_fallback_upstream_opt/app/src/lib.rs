// Links `dep` into the artifact so its exported `swap_generic::<[u64; 16]>`
// is a linked upstream instantiation.
pub use upstream_opt_dep::swap_big;

// Reuses `dep`'s instantiation, which was compiled with `mem::swap` through
// the intrinsic's fallback body (dep is optimizing). `app` is opt-level 0 but
// does not codegen this instance, so the in-place shortcut must not apply.
// That used to show as a rejection at the fallback's UB-check panic; since
// ADR 0008 that panic is out of scope in the default mode and the root
// passes, so this case no longer observes the per-caller rule (it only pins
// that nothing else in the fallback rejects or allocates). Its host twin,
// `strict_host_fallback_upstream_opt`, still does.
#[no_alloc_check::no_alloc]
pub fn shared_swap(a: &mut [u64; 16], b: &mut [u64; 16]) {
    upstream_opt_dep::swap_generic(a, b)
}

// Private and never called, so absent from the codegen units, yet it would be
// codegened here at opt-level 0 if live. `dep` does not instantiate
// `swap_generic::<[u32; 16]>`, so `mem::swap::<[u32; 16]>` is local and lowered
// in place: this must keep passing.
#[allow(dead_code)]
#[no_alloc_check::no_alloc]
fn dead_local(a: &mut [u32; 16], b: &mut [u32; 16]) {
    upstream_opt_dep::swap_generic(a, b)
}
