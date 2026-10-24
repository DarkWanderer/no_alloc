#[no_alloc_check::no_alloc]
pub fn via_dep(a: &mut [u64; 16], b: &mut [u64; 16]) {
    upstream_opt_dep::swap_big(a, b)
}

// Private and never called, so it is absent from the codegen units, yet it
// would be codegened here at opt-level 0 if live: the swap is lowered in
// place and this must keep passing.
#[allow(dead_code)]
#[no_alloc_check::no_alloc]
fn dead_local(a: &mut [u64; 16], b: &mut [u64; 16]) {
    core::mem::swap(a, b)
}
