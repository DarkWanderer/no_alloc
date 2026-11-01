//! `assert_zero_valid` is the one intrinsic family whose lowering depends on
//! the type it is instantiated with, so it is classified per instantiation
//! rather than by name (ADR 0005). This is the rejecting half:
//! `mem::zeroed::<&u8>()` does not meet the requirement, so codegen emits a
//! call to the `panic_nounwind` lang item — synthesized after MIR, where
//! this traversal can no longer see it, and under `panic = "abort"` it
//! reaches a panic handler that allocates.
//!
//! It rejects in every mode by design (ADR 0008): the panic is unconditional
//! and guards UB on every call that reaches it, not a branch of an executable
//! path, so a `Pass` would certify nothing. Not because the panic allocates.
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
