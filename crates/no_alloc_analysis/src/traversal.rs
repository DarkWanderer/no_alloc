//! The production DFS over the exhaustively classified MIR terminator table,
//! memoized via a flat `HashSet<Instance<'tcx>>` (this is what
//! makes the recursive-no-alloc-fn case terminate).
//!
//! **Violation takes priority over rejection within one function.** A
//! single function body routinely has several sequential `Call` terminators
//! (MIR splits a function into one basic block per call, even for
//! straight-line code with no branches at all) — e.g. `alloc::alloc::alloc`
//! calls the no-op stability-shim `__rust_no_alloc_shim_is_unstable_v2`
//! *before* `__rust_alloc`, both as separate, sequential (not
//! alternative-branch) calls. An earlier version of this traversal treated
//! every basic block's terminator as an independent edge and returned on
//! the first non-passing one in iteration order — which meant the shim call
//! (no MIR, not an allocator) was reported as a REJECT before the real
//! allocator call was ever reached, misclassifying the textbook allocation
//! case. Fixed by exploring every edge of a function before deciding: if
//! any edge (or its subtree) is a violation, that wins; only if none is do
//! we fall back to the first rejection found. Caught by the UI test matrix,
//! not by inspection — this is exactly the class of bug the matrix exists
//! to catch (a plausible-looking traversal that's silently wrong).
//!
//! Rejection itself is *not* path-sensitive: a `dyn`/reject edge inside a
//! branch that can never execute at runtime is still found and rejected,
//! because the DFS walks every basic block in the body regardless of
//! whether the branch leading to it is live (see ADR 0003 and the
//! `dead_branch_dyn` fixture).

use crate::leaf::allocates;
use no_alloc_report::{Frame, Verdict};
use rustc_middle::mir::TerminatorKind;
use rustc_middle::ty::{self, Instance, InstanceKind, TyCtxt};
use rustc_span::Span;
use std::collections::HashSet;

/// A checked root instance: the stable, JSON-able `Verdict` plus the raw
/// `Span`s backing its chain (`Verdict::Frame::span` is already a rendered
/// string — a real diagnostic needs the `Span` itself, which
/// `no_alloc_report` can't depend on rustc to hold).
pub struct Checked<'tcx> {
    pub verdict: Verdict,
    pub chain_spans: Vec<(Instance<'tcx>, Span)>,
}

pub fn check_instance<'tcx>(tcx: TyCtxt<'tcx>, root: Instance<'tcx>) -> Checked<'tcx> {
    let mut visited = HashSet::new();
    let mut stack = Vec::new();
    let root_span = tcx.def_span(root.def_id());
    match visit(tcx, root, root_span, &mut visited, &mut stack) {
        None => Checked {
            verdict: Verdict::Pass,
            chain_spans: Vec::new(),
        },
        Some(Finding::Violation(chain)) => Checked {
            verdict: Verdict::Violation {
                chain: to_frames(tcx, &chain),
            },
            chain_spans: chain,
        },
        Some(Finding::Rejected(chain, reason)) => Checked {
            verdict: Verdict::Rejected {
                chain: to_frames(tcx, &chain),
                reason,
            },
            chain_spans: chain,
        },
    }
}

/// Chains are materialized (cloned from `stack`) the instant a finding is
/// discovered, not reconstructed later from a shared mutable stack — that's
/// what makes "keep searching after a rejection, in case a violation turns
/// up elsewhere" safe: the stack keeps changing as the search continues,
/// but an already-captured `Finding` doesn't.
enum Finding<'tcx> {
    Violation(Vec<(Instance<'tcx>, Span)>),
    Rejected(Vec<(Instance<'tcx>, Span)>, String),
}

/// One MIR edge, after resolving whatever the terminator points at.
enum Edge<'tcx> {
    /// No edge here (ignored terminator kind, or a `Drop` on a type that
    /// doesn't need dropping).
    None,
    /// A statically resolved callee to recurse into, entered at `Span` (the
    /// call/drop site in the current instance's body).
    Resolved(Instance<'tcx>, Span),
    /// No statically available callee body — reject, don't assume (ADR 0003).
    Unresolved(String),
}

/// Returns `None` if `instance`'s whole reachable subtree passed (or was
/// already visited — a cycle, or a subtree shared with an earlier branch).
/// `entry_span` is where `instance` was entered from (the root's own
/// `def_span`, or the call/drop site in its caller).
fn visit<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    entry_span: Span,
    visited: &mut HashSet<Instance<'tcx>>,
    stack: &mut Vec<(Instance<'tcx>, Span)>,
) -> Option<Finding<'tcx>> {
    if !visited.insert(instance) {
        return None;
    }
    stack.push((instance, entry_span));

    // Allocator check MUST run before the MIR-availability check: the
    // allocator shims (`__rust_alloc` et al.) are `extern "Rust"`
    // declarations with no body of their own (see leaf.rs) — checking
    // MIR-availability first would silently misclassify a real violation
    // as a rejection.
    if allocates(tcx, instance) {
        let finding = Finding::Violation(stack.clone());
        stack.pop();
        return Some(finding);
    }
    // `Virtual`/`Intrinsic`/`LlvmIntrinsic` instances have no callable MIR
    // of their own — calling `tcx.instance_mir` on `Intrinsic`/
    // `LlvmIntrinsic` is an ICE, not a graceful "no body" signal, so this
    // must be checked (cheaply, no query) before ever reaching that call.
    if matches!(
        instance.def,
        InstanceKind::Virtual(..) | InstanceKind::Intrinsic(_) | InstanceKind::LlvmIntrinsic(_)
    ) || !tcx.is_mir_available(instance.def_id())
        || tcx.is_foreign_item(instance.def_id())
    {
        let finding = Finding::Rejected(
            stack.clone(),
            "no statically available MIR body for this callee".to_string(),
        );
        stack.pop();
        return Some(finding);
    }

    let typing_env = ty::TypingEnv::fully_monomorphized();
    let body = tcx.instance_mir(instance.def);

    let mut pending_reject: Option<Finding<'tcx>> = None;

    for bb in body.basic_blocks.iter() {
        let terminator = bb.terminator();
        let edges = classify_terminator(
            tcx,
            instance,
            typing_env,
            body,
            &terminator.kind,
            terminator.source_info.span,
        );
        for edge in edges {
            match edge {
                Edge::None => {}
                Edge::Resolved(callee, span) => match visit(tcx, callee, span, visited, stack) {
                    Some(Finding::Violation(chain)) => {
                        stack.pop();
                        return Some(Finding::Violation(chain));
                    }
                    Some(finding @ Finding::Rejected(..)) => {
                        pending_reject.get_or_insert(finding);
                    }
                    None => {}
                },
                Edge::Unresolved(reason) => {
                    if pending_reject.is_none() {
                        pending_reject = Some(Finding::Rejected(stack.clone(), reason));
                    }
                }
            }
        }
    }

    stack.pop();
    pending_reject
}

fn classify_terminator<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    kind: &TerminatorKind<'tcx>,
    term_span: Span,
) -> Vec<Edge<'tcx>> {
    let mut edges = Vec::with_capacity(2);
    match kind {
        TerminatorKind::Call { func, .. } | TerminatorKind::TailCall { func, .. } => {
            match func.const_fn_def() {
                Some((def_id, args)) => {
                    let args = instance.instantiate_mir_and_normalize_erasing_regions(
                        tcx,
                        typing_env,
                        ty::EarlyBinder::bind(tcx, args),
                    );
                    match Instance::try_resolve(tcx, typing_env, def_id, args) {
                        Ok(Some(callee)) => edges.push(Edge::Resolved(callee, term_span)),
                        Ok(None) | Err(_) => edges.push(Edge::Unresolved(format!(
                            "callee `{}` could not be resolved to a concrete implementation",
                            tcx.def_path_str(def_id)
                        ))),
                    }
                }
                // Covers fn pointers directly (non-`FnDef` callee type at
                // the MIR level). `dyn` dispatch usually resolves via
                // `try_resolve` to an `InstanceKind::Virtual` instance
                // instead (caught above, not here) — both forms end up
                // rejected, just through the two different guards.
                // Rejection here is deliberately not path-sensitive: this
                // fires even inside a branch that can provably never
                // execute.
                None => edges.push(Edge::Unresolved(
                    "callee is a function pointer, not a statically resolvable body".to_string(),
                )),
            }
        }
        TerminatorKind::Drop { place, .. } => {
            let ty = place.ty(body, tcx).ty;
            let ty = instance.instantiate_mir_and_normalize_erasing_regions(
                tcx,
                typing_env,
                ty::EarlyBinder::bind(tcx, ty),
            );
            if ty.needs_drop(tcx, typing_env) {
                edges.push(Edge::Resolved(
                    Instance::resolve_drop_glue(tcx, ty),
                    term_span,
                ));
            } else {
                edges.push(Edge::None);
            }
        }
        TerminatorKind::InlineAsm { .. } => {
            edges.push(Edge::Unresolved(
                "inline assembly can call or do anything opaquely".to_string(),
            ));
        }
        TerminatorKind::Assert { .. } if tcx.sess.panic_strategy().unwinds() => {
            edges.push(Edge::Unresolved(
                "assertion may invoke the panic handler when panic=unwind".to_string(),
            ));
        }
        TerminatorKind::Assert { .. } => edges.push(Edge::None),
        TerminatorKind::Goto { .. }
        | TerminatorKind::SwitchInt { .. }
        | TerminatorKind::UnwindResume
        | TerminatorKind::UnwindTerminate(_)
        | TerminatorKind::Return
        | TerminatorKind::Unreachable => edges.push(Edge::None),
        TerminatorKind::CoroutineDrop
        | TerminatorKind::Yield { .. }
        | TerminatorKind::FalseEdge { .. }
        | TerminatorKind::FalseUnwind { .. } => edges.push(Edge::Unresolved(
            "unexpected pre-lowering control-flow terminator".to_string(),
        )),
    }

    edges
}

fn to_frames<'tcx>(tcx: TyCtxt<'tcx>, chain: &[(Instance<'tcx>, Span)]) -> Vec<Frame> {
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
