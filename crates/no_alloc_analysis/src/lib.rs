//! rustc-internals analysis: root collection, the allocator leaf predicate,
//! and mono-graph traversal. `diagnostics::emit` turns a checked root
//! into a real rustc diagnostic; `traversal::check_instance` is the
//! production DFS.
#![feature(rustc_private)]

extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;
extern crate rustc_target;

pub mod diagnostics;
pub mod leaf;
pub mod roots;
pub mod traversal;
