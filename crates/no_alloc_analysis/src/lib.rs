//! rustc-internals analysis: root collection, the allocator leaf predicate,
//! and mono-graph traversal.
//!
//! Requires `nightly-2026-08-01` and `rustc_private`. The user-facing entry
//! point is the `cargo-no-alloc` crate.
#![feature(rustc_private)]

extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;

pub mod boundary;
pub mod diagnostics;
pub mod leaf;
pub mod roots;
pub mod traversal;
