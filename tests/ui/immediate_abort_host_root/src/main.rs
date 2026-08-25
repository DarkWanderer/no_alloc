//! The target-side counterpart to `build.rs`, checked under
//! `--immediate-abort`. See `build.rs` for why the two fragments disagreeing
//! on `panic_strategy` (this one, and the host's `unwind`) is not a
//! soundness gap.

#[no_alloc_check::no_alloc]
fn target_trivial(a: u32, b: u32) -> u32 {
    a.wrapping_mul(b)
}

fn main() {
    println!("{}", target_trivial(2, 3));
}
