//! Stable-only report/verdict types and root-spec parsing. Split out from `no_alloc_analysis` so this logic is
//! unit-testable on stable without a `TyCtxt` in scope.

mod report;
mod root_spec;

pub use report::{Frame, Report, ReportFragment, RootVerdict, Verdict};
pub use root_spec::parse_root_spec;
