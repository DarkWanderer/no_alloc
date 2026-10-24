pub fn swap_big(a: &mut [u64; 16], b: &mut [u64; 16]) {
    core::mem::swap(a, b)
}
