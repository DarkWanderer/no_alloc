//! Stable-only report/verdict types, root-spec parsing, the non-allocating
//! intrinsic table, and the panic entry-point table. Split out from
//! `no_alloc_analysis` so this logic is unit-testable on stable without a
//! `TyCtxt` in scope.

mod intrinsic_table;
mod panic_table;
mod reject_reason;
mod report;
mod root_spec;

pub use intrinsic_table::intrinsic_cannot_reach_allocator;
pub use panic_table::is_panic_entry_point;
pub use reject_reason::RejectReason;
pub use report::{
    Environment, Frame, PanicPaths, PanicStrategy, Report, ReportFragment, RootVerdict, Verdict,
    CURRENT_SCHEMA_VERSION,
};
pub use root_spec::parse_root_spec;
