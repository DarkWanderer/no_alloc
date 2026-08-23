//! M5: turns a `Checked` result into a real rustc diagnostic — the chain
//! `root → … → __rust_alloc`, not a bare "this allocates" (path first, span
//! polish is the whole point at mono-site granularity). Errors fail the
//! build; `warn_only` (driven by `NO_ALLOC_WARN_ONLY`) downgrades to
//! warnings for exploration.

use crate::traversal::Checked;
use no_alloc_report::Verdict;
use rustc_middle::ty::TyCtxt;

/// Emits a diagnostic for a non-passing verdict and reports whether it was
/// a hard error (should fail the build — never true when `warn_only`).
pub fn emit<'tcx>(tcx: TyCtxt<'tcx>, checked: &Checked<'tcx>, warn_only: bool) -> bool {
    let headline = match &checked.verdict {
        Verdict::Pass | Verdict::NotInstantiated => return false,
        Verdict::Violation { .. } => {
            "no_alloc: this function may reach the global allocator".to_string()
        }
        Verdict::Rejected { reason, .. } => {
            format!("no_alloc: this function has an unresolved call ({reason})")
        }
    };

    let Some(&(_, root_span)) = checked.chain_spans.first() else {
        // Nothing to anchor a span on (shouldn't happen: a non-Pass verdict
        // always has at least the root's own frame) — nothing sound to emit.
        return false;
    };

    if warn_only {
        let mut diag = tcx.dcx().struct_span_warn(root_span, headline);
        for (instance, span) in &checked.chain_spans {
            diag.span_note(*span, format!("via `{instance}`"));
        }
        diag.emit();
        false
    } else {
        let mut diag = tcx.dcx().struct_span_err(root_span, headline);
        for (instance, span) in &checked.chain_spans {
            diag.span_note(*span, format!("via `{instance}`"));
        }
        diag.emit();
        true
    }
}
