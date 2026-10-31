//! The production DFS over the exhaustively classified MIR terminator table,
//! memoized via a flat set keyed by `(Instance<'tcx>, bool)`, the flag being
//! whether the caller is codegened locally (ADR 0005), so one instance can
//! carry two verdicts; this is what makes the recursive-no-alloc-fn case
//! terminate.
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
//!
//! Two things that look like "no body, therefore reject" are not. An
//! intrinsic is a resolved callee whose body is the code the backend emits
//! for it, so it is classified against the non-allocating intrinsic table
//! (ADR 0005). A compiler-generated shim has a body that `instance_mir`
//! builds on demand, even though `is_mir_available` says no about the
//! `DefId` it stands for — so what has a walkable body is decided by
//! `InstanceKind`, and call edges resolve from the callee operand's *type*
//! rather than from whether it is a MIR constant (ADR 0007).

use crate::leaf::allocates;
use no_alloc_report::{intrinsic_cannot_reach_allocator, Frame, PanicStrategy, Verdict};
use rustc_middle::mir::visit::{PlaceContext, Visitor};
use rustc_middle::mir::{
    Body, Location, Place, ProjectionElem, Rvalue, StatementKind, TerminatorKind,
};
use rustc_middle::ty::layout::ValidityRequirement;
use rustc_middle::ty::{self, Instance, InstanceKind, ShimKind, TyCtxt};
use rustc_span::Span;
use std::collections::HashSet;

/// The panic strategy this compilation is running under, converted at the
/// rustc boundary for the report. What the traversal does with panic paths
/// depends on it (`Assert` above, and whether the panic machinery is even
/// reachable as MIR), so a report that carries a verdict should carry this
/// alongside it — see ADR 0006.
pub fn panic_strategy(tcx: TyCtxt<'_>) -> PanicStrategy {
    match tcx.sess.panic_strategy() {
        rustc_target::spec::PanicStrategy::Unwind => PanicStrategy::Unwind,
        rustc_target::spec::PanicStrategy::Abort => PanicStrategy::Abort,
        rustc_target::spec::PanicStrategy::ImmediateAbort => PanicStrategy::ImmediateAbort,
    }
}

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
    let root_local = tcx.should_codegen_locally(root);
    let root_span = tcx.def_span(root.def_id());
    match visit(tcx, root, root_local, root_span, &mut visited, &mut stack) {
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
    local: bool,
    entry_span: Span,
    visited: &mut HashSet<(Instance<'tcx>, bool)>,
    stack: &mut Vec<(Instance<'tcx>, Span)>,
) -> Option<Finding<'tcx>> {
    if !visited.insert((instance, local)) {
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
    // An intrinsic is fully resolved — the compiler *is* its body — so it is
    // classified against the non-allocating intrinsic table rather than
    // rejected for having no MIR (ADR 0005). Must come before the
    // MIR-availability check below, which would otherwise reject it: calling
    // `tcx.instance_mir` on an `Intrinsic`/`LlvmIntrinsic` is an ICE, not a
    // graceful "no body" signal.
    if let InstanceKind::Intrinsic(def_id) = instance.def {
        let finding = match tcx.intrinsic(def_id) {
            Some(intrinsic) if intrinsic_cannot_reach_allocator(intrinsic.name.as_str()) => {
                // An intrinsic has no body of its own; `local` was passed
                // down unchanged from the body whose MIR contains the call.
                match codegen_fallback(tcx, local, instance, intrinsic) {
                    Some(fallback) => rustc_data_structures::stack::ensure_sufficient_stack(|| {
                        let fallback_local = local && tcx.should_codegen_locally(fallback);
                        visit(
                            tcx,
                            fallback,
                            fallback_local,
                            tcx.def_span(def_id),
                            visited,
                            stack,
                        )
                    }),
                    None => None,
                }
            }
            Some(intrinsic) if ValidityRequirement::from_intrinsic(intrinsic.name).is_some() => {
                classify_validity_assertion(tcx, instance, intrinsic, stack)
            }
            Some(intrinsic) => Some(Finding::Rejected(
                stack.clone(),
                format!(
                    "intrinsic `{}` is not in the non-allocating intrinsic table",
                    intrinsic.name
                ),
            )),
            // `InstanceKind::Intrinsic` without an `IntrinsicDef` shouldn't
            // happen; reject rather than guess if it ever does.
            None => Some(Finding::Rejected(
                stack.clone(),
                "unidentifiable intrinsic callee".to_string(),
            )),
        };
        stack.pop();
        return finding;
    }
    let body = match resolve_body(tcx, instance) {
        Ok(body) => body,
        Err(reason) => {
            let finding = Finding::Rejected(stack.clone(), reason);
            stack.pop();
            return Some(finding);
        }
    };

    let typing_env = ty::TypingEnv::fully_monomorphized();

    let mut pending_reject: Option<Finding<'tcx>> = None;

    // A place projection can lower to a hidden call that no MIR terminator
    // names, so it is audited over the whole body rather than per edge.
    // Recorded as a rejection, not returned, so a real allocator violation
    // elsewhere in this body still takes priority.
    if let Some(reason) = extern_tail_projection(tcx, instance, typing_env, body) {
        pending_reject = Some(Finding::Rejected(stack.clone(), reason));
    }

    for bb in body.basic_blocks.iter() {
        let mut edges = classify_statements(&bb.statements);
        let terminator = bb.terminator();
        edges.extend(classify_terminator(
            tcx,
            instance,
            typing_env,
            body,
            &terminator.kind,
            terminator.source_info.span,
        ));
        for edge in edges {
            match edge {
                Edge::None => {}
                // A std-heavy call graph can nest deeply enough to blow the
                // native stack (this is plain recursion, no depth limit of
                // its own) — `ensure_sufficient_stack` is the same guard
                // rustc's own recursive queries use, growing onto a fresh
                // stack segment before that happens rather than SIGSEGVing
                // the driver.
                Edge::Resolved(callee, span) => {
                    // Mirrors the collector: a callee is codegened here only
                    // if its caller is and `should_codegen_locally` agrees.
                    // An intrinsic is a lowering inside the caller's body.
                    let callee_local = local
                        && (matches!(callee.def, InstanceKind::Intrinsic(_))
                            || tcx.should_codegen_locally(callee));
                    match rustc_data_structures::stack::ensure_sufficient_stack(|| {
                        visit(tcx, callee, callee_local, span, visited, stack)
                    }) {
                        Some(Finding::Violation(chain)) => {
                            stack.pop();
                            return Some(Finding::Violation(chain));
                        }
                        Some(finding @ Finding::Rejected(..)) => {
                            pending_reject.get_or_insert(finding);
                        }
                        None => {}
                    }
                }
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

/// The table in ADR 0005 vouches for the backend's lowering of an intrinsic,
/// but the backend does not always lower it: anything it has no arm for —
/// and `typed_swap_nonoverlapping` for a non-SSA type when optimizing — is
/// codegen'd as a call to the intrinsic's Rust fallback body
/// (`IntrinsicResult::Fallback` in `rustc_codegen_ssa`/`rustc_codegen_llvm`),
/// which is ordinary MIR that can call anything. Whether that happens is
/// decided per call site inside codegen, so this asks the question rustc's
/// own mono collector asks (`visit_instance_use`) to decide whether the
/// fallback must be codegen'd at all: a body exists and the backend has not
/// declared the intrinsic always-replaced. If the collector would emit it,
/// it is walked. A backend that wrongly declared an intrinsic replaced would
/// fail to link rather than silently call the fallback, so the declaration
/// is safe to rely on.
///
/// Other session reads in this traversal, and why they are not subject to
/// the per-caller concern `swap_lowered_in_place` handles:
/// - `panic_strategy` and `replaced_intrinsics`: one value per build graph.
///   rustc refuses to link crates with incompatible panic strategies, and
///   the replaced-intrinsics set is a property of the codegen backend, not
///   of a package profile.
/// - `force_intrinsic_fallback` and `check_validity_requirement`'s
///   `strict_init_checks` are `-Z` flags. rustc does not enforce that they
///   agree across crates (they are only tracked for incremental hashing),
///   but they travel in `RUSTFLAGS`, which `cargo-no-alloc` sets for the
///   whole build, so they are uniform in practice here. A build that set
///   them for only some packages would not be modelled.
///
/// Opt-level, by contrast, is set per package by cargo profile overrides.
fn codegen_fallback<'tcx>(
    tcx: TyCtxt<'tcx>,
    caller_local: bool,
    instance: Instance<'tcx>,
    intrinsic: ty::IntrinsicDef,
) -> Option<Instance<'tcx>> {
    let force = tcx.sess.opts.unstable_opts.force_intrinsic_fallback;
    let may_use_fallback = !intrinsic.must_be_overridden
        && (force || !tcx.sess.replaced_intrinsics.contains(&intrinsic.name))
        && (force || !swap_lowered_in_place(tcx, caller_local, instance, intrinsic));
    may_use_fallback.then(|| Instance::new_raw(instance.def_id(), instance.args))
}

/// `codegen_intrinsic_call`'s one per-instantiation choice among the
/// table's intrinsics: `typed_swap_nonoverlapping` is swapped in place,
/// never through its fallback, for an SSA-standalone type or when not
/// optimizing. Mirrored so `mem::swap` in an unoptimized build is not
/// rejected for a body codegen never calls. A layout that cannot be
/// computed answers "no", which keeps the fallback walked.
///
/// **Whose session decides.** The lowering is chosen while codegening the
/// *caller's* body, in the session of the crate that codegens that body. The
/// layout test is session-independent, so it holds wherever the call is
/// codegened. The opt-level test is not: it is per package profile, and this
/// driver only runs in one crate's session. A non-generic function in a
/// dependency built at `opt-level = 3` can have its MIR walked here at
/// opt-level 0 while its object code took the fallback path. So the
/// `OptLevel::No` shortcut applies only when the caller is codegened in this
/// crate (`caller_local`). That is rustc's own `should_codegen_locally`
/// decision, applied the way the collector applies it along the walk: a root
/// is local if it is a local definition (reachable or not), and a callee is
/// local only if its caller is and `should_codegen_locally` holds for it,
/// which is false for an upstream non-generic and for an upstream-shared
/// instantiation. The query alone is not enough: an `#[inline]` generic such
/// as `mem::swap::<T>` is local by that test, yet is only emitted in the
/// crate that reaches it, which for a dependency's body is the dependency.
/// Any other caller walks the fallback, which is the conservative direction
/// (ADR 0003, ADR 0005).
fn swap_lowered_in_place<'tcx>(
    tcx: TyCtxt<'tcx>,
    caller_local: bool,
    instance: Instance<'tcx>,
    intrinsic: ty::IntrinsicDef,
) -> bool {
    if intrinsic.name != rustc_span::sym::typed_swap_nonoverlapping {
        return false;
    }
    let typing_env = ty::TypingEnv::fully_monomorphized();
    tcx.layout_of(typing_env.as_query_input(instance.args.type_at(0)))
        .is_ok_and(|layout| layout.is_ssa_standalone())
        || (caller_local && tcx.sess.opts.optimize == rustc_session::config::OptLevel::No)
}

/// `assert_inhabited`, `assert_zero_valid` and `assert_mem_uninitialized_valid`
/// are the one family whose lowering depends on the instantiation, so they
/// cannot be settled by the name table: codegen asks whether *this* type
/// meets the requirement and emits either nothing at all or a call to the
/// `panic_nounwind` lang item — a Rust function, synthesized after MIR, that
/// this traversal would never see (`rustc_codegen_ssa`'s
/// `codegen_panic_intrinsic`).
///
/// So this asks codegen's own question, with codegen's own query, and
/// classifies per instantiation (ADR 0001, ADR 0005). A type that meets the
/// requirement compiles to no code and is terminal; one that does not is
/// rejected, because the panic it compiles to is exactly the edge the
/// traversal cannot follow. A type whose layout cannot be computed is
/// rejected too — no answer is not a "yes".
fn classify_validity_assertion<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    intrinsic: ty::IntrinsicDef,
    stack: &[(Instance<'tcx>, Span)],
) -> Option<Finding<'tcx>> {
    let requirement = ValidityRequirement::from_intrinsic(intrinsic.name)?;
    let ty = instance.args.type_at(0);
    let typing_env = ty::TypingEnv::fully_monomorphized();
    let holds = tcx
        .check_validity_requirement((requirement, typing_env.as_query_input(ty)))
        .unwrap_or(false);
    if holds {
        return None;
    }
    Some(Finding::Rejected(
        stack.to_vec(),
        format!(
            "`{}::<{ty}>` does not hold, so this call compiles to a panic",
            intrinsic.name
        ),
    ))
}

/// Codegen's `project_field` (`rustc_codegen_ssa/src/mir/place.rs`) computes
/// the alignment of an unsized field at a non-zero offset with
/// `size_of_val::size_and_align_of_dst`, whose `ty::Foreign` arm emits a call
/// to the `panic_nounwind` lang item: a Rust call that exists in no MIR body,
/// so the traversal would never see it. Mirroring the exact offset rule would
/// mean depending on layout details, so any field projection whose type has
/// an extern-type tail is rejected, whatever its offset (ADR 0003: reject,
/// don't assume).
fn extern_tail_projection<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    body: &Body<'tcx>,
) -> Option<String> {
    struct Finder<'tcx> {
        tcx: TyCtxt<'tcx>,
        instance: Instance<'tcx>,
        typing_env: ty::TypingEnv<'tcx>,
        found: bool,
    }
    impl<'tcx> Visitor<'tcx> for Finder<'tcx> {
        fn visit_place(&mut self, place: &Place<'tcx>, _: PlaceContext, _: Location) {
            for elem in place.projection.iter() {
                let ProjectionElem::Field(_, field_ty) = elem else {
                    continue;
                };
                let field_ty = self.instance.instantiate_mir_and_normalize_erasing_regions(
                    self.tcx,
                    self.typing_env,
                    ty::EarlyBinder::bind(self.tcx, field_ty),
                );
                let tail = self.tcx.struct_tail_for_codegen(field_ty, self.typing_env);
                if matches!(tail.kind(), ty::Foreign(_)) {
                    self.found = true;
                }
            }
        }
    }
    let mut finder = Finder {
        tcx,
        instance,
        typing_env,
        found: false,
    };
    finder.visit_body(body);
    finder.found.then(|| {
        "projection into an extern-type tail: codegen computes its size with a `panic_nounwind` call"
            .to_string()
    })
}

fn has_explicit_linkage(tcx: TyCtxt<'_>, def_id: rustc_hir::def_id::DefId) -> bool {
    let attrs = tcx.codegen_fn_attrs(def_id);
    attrs.linkage.is_some() || attrs.import_linkage.is_some()
}

const NO_MIR_BODY: &str = "no statically available MIR body for this callee";

/// Exhaustively classifies whether an instance has faithful MIR to traverse.
/// This must be decided from `InstanceKind`, not from `instance.def_id()`:
/// shims have synthesized MIR even when the item whose `DefId` they carry is
/// bodiless. Future rustc variants intentionally fail this match at compile
/// time until their lowering has been audited (ADR 0003).
fn resolve_body<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
) -> Result<&'tcx Body<'tcx>, String> {
    match instance.def {
        InstanceKind::Item(def_id) => {
            if tcx.is_foreign_item(def_id) || !tcx.is_mir_available(def_id) {
                Err(NO_MIR_BODY.to_string())
            } else if has_explicit_linkage(tcx, def_id) {
                // `#[linkage = "weak"]` and friends: the MIR here is one
                // candidate definition, but the linker may bind the symbol
                // to a different one, so its MIR is not faithful evidence.
                Err(
                    "callee has explicit linkage; the linker may bind it to a different definition"
                        .to_string(),
                )
            } else {
                Ok(tcx.instance_mir(instance.def))
            }
        }
        // Rust intrinsics return before this function after classification
        // against ADR 0005. Keep the fallback rejecting so refactoring that
        // path cannot turn an intrinsic into an `instance_mir` ICE.
        InstanceKind::Intrinsic(_) | InstanceKind::LlvmIntrinsic(_) => Err(NO_MIR_BODY.to_string()),
        InstanceKind::Virtual(..) => Err(NO_MIR_BODY.to_string()),
        InstanceKind::Shim(shim) => resolve_shim_body(tcx, instance, shim),
    }
}

/// Per-shim traverse/reject decision, checked against
/// `rustc_mir_transform::shim::make_shim` for the pinned nightly. Although
/// rustc can synthesize MIR for every shim, only bodies whose lowering has
/// been audited are safe to treat as faithful call-graph evidence.
fn resolve_shim_body<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
    shim: ShimKind<'tcx>,
) -> Result<&'tcx Body<'tcx>, String> {
    match shim {
        // Direct-call adapters whose synthesized bodies expose their real
        // callees to the ordinary `Call` classification below.
        ShimKind::VTable(_)
        | ShimKind::Reify(..)
        | ShimKind::FnPtr(..)
        | ShimKind::ClosureOnce { .. }
        | ShimKind::DropGlue(..)
        | ShimKind::Clone(..)
        | ShimKind::FnPtrAddr(..) => Ok(tcx.instance_mir(instance.def)),
        // TLS access can lower to a runtime call that has no MIR callee.
        ShimKind::ThreadLocal(_) => Err(
            "thread-local access is not modeled (may call into the TLS runtime on some targets)"
                .to_string(),
        ),
        // These unstable synthesized bodies have not been audited.
        ShimKind::ConstructCoroutineInClosure { .. } => {
            Err("coroutine-closure construction shim is not yet audited by this tool".to_string())
        }
        ShimKind::FutureDropPoll(..)
        | ShimKind::AsyncDropGlue(..)
        | ShimKind::AsyncDropGlueCtor(..) => {
            Err("async drop glue is not yet audited by this tool".to_string())
        }
    }
}

/// Non-terminator MIR (`StatementKind`/`Rvalue`) audit (F3 in the soundness
/// review): every variant of both is call-free by construction — pure data
/// movement, casts, comparisons, or aggregate/discriminant computation, none
/// of which can reach the allocator — with one documented exception.
///
/// `Rvalue::ThreadLocalRef` is the only place where a *statement* (not a
/// terminator) can execute real code: rustc's own doc comment on it says so
/// directly ("this is a runtime operation that actually executes code and
/// is in this sense more like a function call"), because on some TLS models
/// it lowers to a call to `__tls_get_addr`, which glibc can route through
/// `malloc` on first access to a `dlopen`ed module. There is no MIR callee
/// to follow — it is not a `Call`, so nothing here can name a resolvable
/// `Instance` for it the way every other edge in this traversal does — so
/// it is rejected outright rather than silently treated as a plain data
/// read. This is the same conservative call made for `ShimKind::ThreadLocal`
/// (`resolve_shim_body`, above): a cross-crate thread-local access goes
/// through that shim, a same-crate one is this bare statement, and neither
/// is modeled beyond "reject, don't assume".
///
/// `StatementKind::Intrinsic(NonDivergingIntrinsic::Assume | CopyNonOverlapping)`
/// and every other `StatementKind` (`Assign` with a non-`ThreadLocalRef`
/// `Rvalue`, `FakeRead`, `SetDiscriminant`, `StorageLive`/`StorageDead`,
/// `PlaceMention`, `AscribeUserType`, `Coverage`, `ConstEvalCounter`, `Nop`,
/// `BackwardIncompatibleDropHint`) were checked against their doc comments
/// in `rustc_middle::mir::syntax` and confirmed call-free.
///
/// Both matches below name every variant instead of falling back to `_`, on
/// purpose: a `_` arm would keep compiling — and silently return `None`, the
/// "no edge" answer — the day the pinned nightly adds a new `StatementKind`
/// or `Rvalue` variant that was never audited against this reasoning. Naming
/// them all turns that into a compile error here instead, which is the only
/// way to *force* the audit rather than hope someone remembers it.
fn classify_statements<'tcx>(statements: &[rustc_middle::mir::Statement<'tcx>]) -> Vec<Edge<'tcx>> {
    statements
        .iter()
        .filter_map(|statement| match &statement.kind {
            StatementKind::Assign(assign) => match assign.1 {
                Rvalue::ThreadLocalRef(_) => Some(Edge::Unresolved(
                    "thread-local access is not modeled (may call into the TLS runtime on some targets)"
                        .to_string(),
                )),
                Rvalue::Use(..)
                | Rvalue::Repeat(..)
                | Rvalue::Ref(..)
                | Rvalue::RawPtr(..)
                | Rvalue::Cast(..)
                | Rvalue::BinaryOp(..)
                | Rvalue::UnaryOp(..)
                | Rvalue::Discriminant(..)
                | Rvalue::Aggregate(..)
                | Rvalue::CopyForDeref(..)
                | Rvalue::WrapUnsafeBinder(..)
                | Rvalue::Reborrow(..) => None,
            },
            StatementKind::FakeRead(..)
            | StatementKind::SetDiscriminant { .. }
            | StatementKind::StorageLive(..)
            | StatementKind::StorageDead(..)
            | StatementKind::PlaceMention(..)
            | StatementKind::AscribeUserType(..)
            | StatementKind::Coverage(..)
            | StatementKind::Intrinsic(..)
            | StatementKind::ConstEvalCounter
            | StatementKind::Nop
            | StatementKind::BackwardIncompatibleDropHint { .. } => None,
        })
        .collect()
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
            // Classified by the callee operand's *type*, not by whether it
            // happens to be a MIR constant. A `FnDef` type names exactly one
            // function and is zero-sized, so a callee that arrives as a move
            // out of a local (which is how compiler-generated shims call the
            // function they were built for) is as resolvable as one written
            // as a literal path — `Operand::const_fn_def` sees only the
            // latter, and rejecting the former cost the traversal every
            // callback passed as a function item (ADR 0007).
            let callee_ty = instance.instantiate_mir_and_normalize_erasing_regions(
                tcx,
                typing_env,
                ty::EarlyBinder::bind(tcx, func.ty(body, tcx)),
            );
            match *callee_ty.kind() {
                // `no_bound_vars` holds for anything reached from a
                // monomorphized root; rejecting rather than unwrapping keeps
                // a surprise here a finding instead of an ICE.
                ty::FnDef(def_id, args) => match args.no_bound_vars() {
                    Some(args) => match Instance::try_resolve(tcx, typing_env, def_id, args) {
                        Ok(Some(callee)) => edges.push(Edge::Resolved(callee, term_span)),
                        Ok(None) | Err(_) => edges.push(Edge::Unresolved(format!(
                            "callee `{}` could not be resolved to a concrete implementation",
                            tcx.def_path_str(def_id)
                        ))),
                    },
                    None => edges.push(Edge::Unresolved(format!(
                        "callee `{}` is still generic over bound variables",
                        tcx.def_path_str(def_id)
                    ))),
                },
                // A genuine `fn` pointer: the type names a signature, not a
                // body. `dyn` dispatch usually resolves via `try_resolve` to
                // an `InstanceKind::Virtual` instance instead (caught in
                // `visit`, not here) — both forms end up rejected, just
                // through the two different guards. Rejection here is
                // deliberately not path-sensitive: this fires even inside a
                // branch that can provably never execute.
                _ => edges.push(Edge::Unresolved(
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
            if let ty::Dynamic(..) = ty.kind() {
                // `Instance::resolve_drop_glue(tcx, dyn Trait)` resolves to
                // `Shim(DropGlue(drop_in_place, Some(dyn Trait)))`, whose
                // *underlying* def_id (`drop_in_place`) owns a real MIR
                // body, so the `is_mir_available` guard below never fires
                // for it. But the shim's actual body — synthesized by the
                // drop elaborator (`elaborate_drop.rs`'s `Dynamic` arm) —
                // is a residual `Drop` terminator on the same still-`dyn`
                // place, which resolves to the *same* instance and gets
                // swallowed by `visited` as a silent pass (F0 in the
                // soundness review). Codegen never takes this path: it
                // replaces the resolved glue with
                // `InstanceKind::Virtual(drop_fn.def_id(), 0)` and calls
                // the concrete destructor through the vtable's drop slot
                // (`rustc_codegen_ssa/src/mir/block.rs`). That concrete
                // `Drop::drop` is not statically known here, so reject —
                // this also catches the shim's residual `Drop`, since it
                // has the same `dyn` place type and takes this same arm.
                edges.push(Edge::Unresolved(
                    "drop through a vtable: the concrete destructor is not statically known"
                        .to_string(),
                ));
            } else if ty.needs_drop(tcx, typing_env) {
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
        // Under immediate-abort there is no carve-out (ADR 0006): codegen
        // lowers a failing assertion to a call to a core panic lang item,
        // and that call is followed like any other, rather than trusted to
        // be the bare `abort` std compiles it to in this mode.
        TerminatorKind::Assert { msg, .. }
            if tcx.sess.panic_strategy() == rustc_target::spec::PanicStrategy::ImmediateAbort =>
        {
            edges.push(assert_panic_edge(tcx, msg, term_span));
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

/// The lang item `codegen_assert_terminator` (`rustc_codegen_ssa`) calls
/// when an assertion fails, chosen exactly as it chooses it. Lang items are
/// non-generic, so codegen calls their `Instance::mono`, and so does this.
fn assert_panic_edge<'tcx>(
    tcx: TyCtxt<'tcx>,
    msg: &rustc_middle::mir::AssertMessage<'tcx>,
    span: Span,
) -> Edge<'tcx> {
    use rustc_hir::LangItem;
    use rustc_middle::mir::AssertKind;
    let lang_item = match msg {
        AssertKind::BoundsCheck { .. } => LangItem::PanicBoundsCheck,
        AssertKind::MisalignedPointerDereference { .. } => {
            LangItem::PanicMisalignedPointerDereference
        }
        AssertKind::NullPointerDereference => LangItem::PanicNullPointerDereference,
        AssertKind::NullReferenceConstructed => LangItem::PanicNullReferenceConstructed,
        AssertKind::InvalidEnumConstruction(_) => LangItem::PanicInvalidEnumConstruction,
        _ => msg.panic_function(),
    };
    match tcx.lang_items().get(lang_item) {
        Some(def_id) => Edge::Resolved(Instance::mono(tcx, def_id), span),
        None => Edge::Unresolved(format!(
            "assertion panics through lang item `{}`, which is not defined",
            lang_item.name()
        )),
    }
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
