//! The one place rustc types become `no_alloc_report` types. The report crate
//! must stay buildable on stable, so every conversion lives here and nowhere
//! else; callers elsewhere in this crate and in the driver only see the
//! report-side values.

use crate::roots::root_path;
use no_alloc_report::{Environment, Frame, PanicStrategy, RootVerdict, Verdict};
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{Instance, TyCtxt};
use rustc_session::config::OptLevel;
use rustc_span::Span;

/// The panic strategy this compilation is running under. What the traversal
/// does with panic paths depends on it (`Assert`, and whether the panic
/// machinery is even reachable as MIR), so a report that carries a verdict
/// should carry this alongside it — see ADR 0006. Exhaustive so a new rustc
/// strategy fails to compile here instead of being relabelled.
pub fn panic_strategy(tcx: TyCtxt<'_>) -> PanicStrategy {
    match tcx.sess.panic_strategy() {
        rustc_target::spec::PanicStrategy::Unwind => PanicStrategy::Unwind,
        rustc_target::spec::PanicStrategy::Abort => PanicStrategy::Abort,
        rustc_target::spec::PanicStrategy::ImmediateAbort => PanicStrategy::ImmediateAbort,
    }
}

/// Spelled out instead of formatted with `{:?}` so that a renamed or added
/// rustc variant is a compile error here rather than a silent change to the
/// strings recorded in `report.json`. The spellings are what the derived
/// `Debug` impl produced when the report schema was written.
fn opt_level(level: OptLevel) -> &'static str {
    match level {
        OptLevel::No => "No",
        OptLevel::Less => "Less",
        OptLevel::More => "More",
        OptLevel::Aggressive => "Aggressive",
        OptLevel::Size => "Size",
        OptLevel::SizeMin => "SizeMin",
    }
}

/// `all_crates` and `build_std` are invocation-level flags that are not
/// visible on `Session`, so the driver reads them from its environment and
/// passes them in.
pub fn environment(tcx: TyCtxt<'_>, all_crates: bool, build_std: bool) -> Environment {
    let sess = &tcx.sess;
    Environment {
        panic_strategy: panic_strategy(tcx),
        opt_level: opt_level(sess.opts.optimize).to_string(),
        mir_opt_level: sess.mir_opt_level() as u32,
        overflow_checks: sess.overflow_checks(),
        debug_assertions: sess.opts.debug_assertions,
        target_triple: sess.opts.target_triple.tuple().to_string(),
        rustc_version: rustc_interface::util::rustc_version_str()
            .unwrap_or("unknown")
            .to_string(),
        all_crates,
        build_std,
    }
}

/// A root marker with no instance to check: it has no verdict-bearing
/// instance string and no environment.
pub fn not_instantiated_verdict(tcx: TyCtxt<'_>, root: DefId) -> RootVerdict {
    let path = root_path(tcx, root);
    RootVerdict {
        root: path.clone(),
        instance: path,
        verdict: Verdict::NotInstantiated,
        environment: None,
    }
}

/// A checked root: the verdict is per instantiation, so the concrete
/// `instance` string is recorded next to the definition path.
pub fn root_verdict<'tcx>(
    tcx: TyCtxt<'tcx>,
    root: DefId,
    instance: Instance<'tcx>,
    verdict: Verdict,
    environment: Environment,
) -> RootVerdict {
    RootVerdict {
        root: root_path(tcx, root),
        instance: instance.to_string(),
        verdict,
        environment: Some(environment),
    }
}

pub fn to_frames<'tcx>(tcx: TyCtxt<'tcx>, chain: &[(Instance<'tcx>, Span)]) -> Vec<Frame> {
    chain
        .iter()
        .map(|(instance, span)| Frame {
            def_path: tcx.def_path_str(instance.def_id()),
            // Same rendering `diagnostics.rs` uses for its `via `{instance}``
            // notes, so report.json's chain matches the stderr it's meant to
            // corroborate (the whole point being a per-instantiation verdict
            // needs the concrete type arguments, not just the definition).
            instance: instance.to_string(),
            span: Some(tcx.sess.source_map().span_to_diagnostic_string(*span)),
        })
        .collect()
}
