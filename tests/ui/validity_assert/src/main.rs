//! `assert_zero_valid` is the one intrinsic family whose lowering depends on
//! the type it is instantiated with, so it is classified per instantiation
//! rather than by name (ADR 0005). This is the rejecting half:
//! `mem::zeroed::<&u8>()` does not meet the requirement, so codegen emits a
//! call to the `panic_nounwind` lang item — synthesized after MIR, where
//! this traversal can no longer see it, and under `panic = "abort"` it
//! reaches a panic handler that allocates.
//!
//! This still rejects in the default mode (ADR 0008): the panic call is
//! synthesized after MIR, so it is not a call to a panic entry point the
//! traversal could recognise, and is treated like any other unresolved edge.
//!
//! The passing half is `strict_iterator::search`, under `--strict`.

#[no_alloc_check::no_alloc]
#[allow(invalid_value)]
fn not_zeroable() -> usize {
    // SAFETY: none — this is instant UB, and the point of the fixture. The
    // checker's job here is to notice the panic the compiler emits for it,
    // not to be handed a well-formed program.
    let reference: &u8 = unsafe { std::mem::zeroed() };
    reference as *const u8 as usize
}

fn main() {
    println!("{}", not_zeroable());
}
