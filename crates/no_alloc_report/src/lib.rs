//! Report and verdict types, root-spec parsing, and the audited
//! non-allocating intrinsic and panic entry-point tables for `cargo-no-alloc`.

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
