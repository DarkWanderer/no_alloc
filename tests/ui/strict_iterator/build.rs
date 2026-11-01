//! Exists to put a *host* compilation in this fixture's build.
//!
//! Cargo does not pass the target `RUSTFLAGS` to build scripts when
//! `--target` is set, so this unit compiles under the ordinary unwinding
//! strategy while everything checked compiles under immediate-abort. It is
//! still a workspace member, so the checker's driver wraps it and it writes
//! a report fragment — one that carries no verdicts. If that fragment were
//! allowed to state a panic strategy, it would disagree with the real ones
//! and the merged report would have to give up and say nothing.
fn main() {
    println!("cargo::rerun-if-changed=build.rs");
}
