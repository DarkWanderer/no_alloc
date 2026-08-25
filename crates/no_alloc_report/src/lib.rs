//! Stable-only report/verdict types, root-spec parsing, and the
//! non-allocating intrinsic table. Split out from `no_alloc_analysis` so
//! this logic is unit-testable on stable without a `TyCtxt` in scope.

mod intrinsic_table;
mod report;
mod root_spec;

pub use intrinsic_table::intrinsic_cannot_reach_allocator;
pub use report::{Frame, PanicStrategy, Report, ReportFragment, RootVerdict, Verdict};
pub use root_spec::parse_root_spec;
