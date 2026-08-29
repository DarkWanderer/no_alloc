//! Pins the reasoning in ADR 0006's "Host units always compile under
//! `unwind`" section. A build script is a *host* artifact, so Cargo
//! compiles it under the default `unwind` strategy regardless of
//! `--immediate-abort` — confirmed here by this fragment's own
//! `panic_strategy` disagreeing with the target's.
//!
//! That disagreement is not a soundness gap. `unwind` is the *stricter* of
//! the two panic strategies as far as this traversal is concerned: its
//! `Assert` handling is a hard rejection (ADR 0003), never the `Edge::None`
//! carve-out `abort`/`immediate-abort` give that terminator. So `Pass` on a
//! host root proves at least as much as `Pass` on a target root — and if it
//! didn't (a genuine violation or rejection), `diagnostics::emit` fails the
//! build script's own compilation outright, which fails the whole build
//! before the target crate is even reached. There is no route to a
//! successful build with an unsound host verdict hiding inside it.

#[no_alloc_check::no_alloc]
fn host_trivial(a: u32, b: u32) -> u32 {
    a.wrapping_mul(b)
}

fn main() {
    println!("cargo::rerun-if-changed=build.rs {}", host_trivial(2, 3));
}
