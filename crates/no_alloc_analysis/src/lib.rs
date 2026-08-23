//! rustc-internals analysis: root collection, the allocator leaf predicate,
//! and mono-graph traversal. M5: `diagnostics::emit` turns a checked root
//! into a real rustc diagnostic; `traversal::check_instance` is the
//! production DFS; `probe` stays only as the M3 leaf-predicate proof.
#![feature(rustc_private)]

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

pub mod diagnostics;
pub mod leaf;
pub mod mono_dump;
pub mod probe;
pub mod roots;
pub mod traversal;
