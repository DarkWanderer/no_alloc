//! The allocator/deallocator leaf predicate — the one thing M3 exists to
//! prove correct in isolation before it gets buried in a traversal.
//!
//! Checked purely via `codegen_fn_attrs` flags, never by symbol name (see
//! ADR — no symbol-name string matching, ever). Deallocation counts: it
//! takes the same lock as allocation.
//!
//! Ordering note, confirmed at M3 against `library/alloc/src/alloc.rs`:
//! `__rust_alloc`/`__rust_dealloc`/`__rust_realloc`/`__rust_alloc_zeroed`
//! are declared inside `unsafe extern "Rust" { ... }` — they have **no
//! MIR body** in this crate (the real body is generated later, by a
//! `#[global_allocator]` shim or std's `__rdl_alloc`). A traversal MUST
//! check `allocates` before any "no MIR body -> reject" check, or these
//! real violations get silently misclassified as rejections. This is
//! exactly the silent-failure mode that would make the tool unsound while
//! appearing to work.

use rustc_middle::middle::codegen_fn_attrs::CodegenFnAttrFlags;
use rustc_middle::ty::{Instance, TyCtxt};

const ALLOC_FLAGS: CodegenFnAttrFlags = CodegenFnAttrFlags::ALLOCATOR
    .union(CodegenFnAttrFlags::DEALLOCATOR)
    .union(CodegenFnAttrFlags::REALLOCATOR)
    .union(CodegenFnAttrFlags::ALLOCATOR_ZEROED);

pub fn allocates<'tcx>(tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
    tcx.codegen_fn_attrs(instance.def_id())
        .flags
        .intersects(ALLOC_FLAGS)
}
