//! Stable-only report/verdict types, root-spec parsing, sidecar serde, and
//! path rendering. Split out from `no_alloc_analysis` so this logic is
//! unit-testable on stable without a `TyCtxt` in scope.

mod report;
mod root_spec;
mod sidecar;

pub use report::{Frame, Report, RootVerdict, Verdict};
pub use root_spec::parse_root_spec;
pub use sidecar::{DefPathHash, RootIndex, SIDECAR_DIR};
