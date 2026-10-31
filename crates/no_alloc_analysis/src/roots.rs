//! Root discovery on the complete monomorphized instance graph.

use crate::traversal::{resolve_body, resolved_callees};
use no_alloc_report::parse_root_spec;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_middle::mono::MonoItem;
use rustc_middle::ty::{Instance, InstanceKind, TyCtxt};
use rustc_span::Symbol;
use std::collections::HashSet;

fn root_attr_path() -> [Symbol; 2] {
    [Symbol::intern("no_alloc_tool"), Symbol::intern("root")]
}

pub fn is_root(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.get_attrs_by_path(def_id, &root_attr_path())
        .next()
        .is_some()
}

pub fn root_path(tcx: TyCtxt<'_>, def_id: DefId) -> String {
    let path = tcx.def_path_str(def_id);
    if def_id.is_local() {
        format!(
            "{}::{path}",
            tcx.crate_name(rustc_span::def_id::LOCAL_CRATE)
        )
    } else {
        path
    }
}

fn record_matches(
    tcx: TyCtxt<'_>,
    def_id: DefId,
    requested: &HashSet<String>,
    matched: &mut HashSet<String>,
) -> bool {
    let raw = tcx.def_path_str(def_id);
    let canonical = root_path(tcx, def_id);
    let mut selected = false;
    for candidate in [raw, canonical] {
        if requested.contains(&candidate) {
            matched.insert(candidate);
            selected = true;
        }
    }
    selected
}

#[derive(Clone, Copy)]
pub enum DiscoveredRoot<'tcx> {
    Instance {
        root: DefId,
        instance: Instance<'tcx>,
    },
    NotInstantiated {
        root: DefId,
    },
}

pub struct Discovery<'tcx> {
    pub roots: Vec<DiscoveredRoot<'tcx>>,
    pub matched_root_specs: Vec<String>,
    pub selection_errors: Vec<String>,
}

pub fn discover<'tcx>(tcx: TyCtxt<'tcx>) -> Discovery<'tcx> {
    let requested: HashSet<String> = std::env::var("NO_ALLOC_ROOTS")
        .ok()
        .map(|value| parse_root_spec(&value).into_iter().collect())
        .unwrap_or_default();
    let mut matched = HashSet::new();
    let mut selection_errors = Vec::new();
    let mut roots = Vec::new();
    let mut seen_instances = HashSet::new();
    let mut instantiated_defs = HashSet::new();

    let partitions = tcx.collect_and_partition_mono_items(());
    for item in partitions
        .codegen_units
        .iter()
        .flat_map(|cgu| cgu.items().keys())
    {
        let MonoItem::Fn(instance) = item else {
            continue;
        };
        let def_id = instance.def_id();
        let selected = record_matches(tcx, def_id, &requested, &mut matched);
        if !(selected || is_root(tcx, def_id)) {
            continue;
        }
        // Reported as a selection error below instead of a verdict on the
        // future constructor. Gated on the def kind and on being a root
        // candidate because `asyncness` is only encoded in crate metadata
        // for `Fn`/`AssocFn`; querying it for every mono item would read a
        // missing table entry for upstream closures and coroutines.
        if matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn)
            && tcx.asyncness(def_id).is_async()
        {
            continue;
        }
        if seen_instances.insert(*instance) {
            instantiated_defs.insert(def_id);
            roots.push(DiscoveredRoot::Instance {
                root: def_id,
                instance: *instance,
            });
        }
    }

    discover_upstream_shared(tcx, &mut seen_instances, &mut instantiated_defs, &mut roots);

    // Mono collection cannot contain uncalled local functions. Seed those
    // explicitly, while preserving NotInstantiated for generic definitions.
    for local in tcx.iter_local_def_id() {
        let def_id = local.to_def_id();
        let path = root_path(tcx, def_id);
        let selected = record_matches(tcx, def_id, &requested, &mut matched);
        let marked = selected || is_root(tcx, def_id);
        // Checked for *either* source of "this is a root candidate", not
        // just `--root`-selection: a hand-written `#[no_alloc_tool::root]`
        // (the internal attribute `#[no_alloc_check::no_alloc]` expands to)
        // placed directly on a non-function item used to reach
        // `tcx.generics_of`/`Instance::mono` below unguarded — verified this
        // silently misclassified a `static` as a passing root instance
        // rather than reporting it (a `tcx.instance_mir` call against its
        // CTFE body, not a real function); other `DefKind`s are plausibly an
        // ICE surface instead. Either way, this is a case the macro itself
        // already rejects when reached through it normally.
        if marked && !matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn) {
            let source = if selected { "requested" } else { "marked" };
            selection_errors.push(format!("{source} root `{path}` is not a function"));
            continue;
        }
        // An `async fn`'s own MIR is only the future constructor; the body
        // that runs lives in a separate coroutine, which would pass
        // unchecked. Rejected rather than followed (ADR 0003).
        if marked && tcx.asyncness(def_id).is_async() {
            let source = if selected { "requested" } else { "marked" };
            selection_errors.push(format!(
                "{source} root `{path}` is an async fn (only its future constructor would be checked)"
            ));
            continue;
        }
        if !marked || instantiated_defs.contains(&def_id) {
            continue;
        }
        // A generic root that MIR inlining removed from every caller is
        // reported here. The `#[no_alloc]` marker also emits
        // `rustc_no_mir_inline` so a marked root survives as a mono item;
        // a `--root`-selected (unmarked) generic fn has no such protection
        // and remains exposed to this false `NotInstantiated`.
        if tcx.generics_of(def_id).requires_monomorphization(tcx) {
            roots.push(DiscoveredRoot::NotInstantiated { root: def_id });
        } else {
            let instance = Instance::mono(tcx, def_id);
            if seen_instances.insert(instance) {
                roots.push(DiscoveredRoot::Instance {
                    root: def_id,
                    instance,
                });
            }
        }
    }

    let mut matched_root_specs: Vec<_> = matched.into_iter().collect();
    matched_root_specs.sort();
    selection_errors.sort();
    roots.sort_by_key(|root| match root {
        DiscoveredRoot::Instance { root, instance } => {
            (root_path(tcx, *root), instance.to_string())
        }
        DiscoveredRoot::NotInstantiated { root } => {
            let path = root_path(tcx, *root);
            (path.clone(), path)
        }
    });
    Discovery {
        roots,
        matched_root_specs,
        selection_errors,
    }
}

/// A generic instantiation that an upstream crate already emitted is shared
/// (`-Zshare-generics`, on by default at opt-level 0 and always for
/// `#[inline(never)]`), so the local collector links to it instead of
/// listing it among the mono items. A marked root instantiated that way would
/// silently vanish from discovery. This walks the call graph from every local
/// mono item the way the collector does, entering only callees that are not
/// codegened locally (those already are mono items), and adopts any marked
/// root it meets.
fn discover_upstream_shared<'tcx>(
    tcx: TyCtxt<'tcx>,
    seen_instances: &mut HashSet<Instance<'tcx>>,
    instantiated_defs: &mut HashSet<DefId>,
    roots: &mut Vec<DiscoveredRoot<'tcx>>,
) {
    let partitions = tcx.collect_and_partition_mono_items(());
    let mut worklist: Vec<Instance<'tcx>> = partitions
        .codegen_units
        .iter()
        .flat_map(|cgu| cgu.items().keys())
        .filter_map(|item| match item {
            MonoItem::Fn(instance) => Some(*instance),
            _ => None,
        })
        .collect();
    let mut visited: HashSet<Instance<'tcx>> = worklist.iter().copied().collect();
    while let Some(instance) = worklist.pop() {
        // A body `resolve_body` cannot give is skipped silently: a real root
        // that reaches it still reports the rejection.
        let Ok(body) = resolve_body(tcx, instance) else {
            continue;
        };
        for callee in resolved_callees(tcx, instance, body) {
            if !matches!(callee.def, InstanceKind::Item(_) | InstanceKind::Shim(_))
                || tcx.should_codegen_locally(callee)
                || !visited.insert(callee)
            {
                continue;
            }
            let def_id = callee.def_id();
            // Only generic instantiations: those are what the local collector
            // would have codegened (and so discovered) but for sharing. A
            // non-generic marked root in a non-workspace dependency stays
            // unchecked without `--all-crates`, as ADR 0004 documents.
            if is_root(tcx, def_id)
                && tcx.generics_of(def_id).requires_monomorphization(tcx)
                && seen_instances.insert(callee)
            {
                instantiated_defs.insert(def_id);
                roots.push(DiscoveredRoot::Instance {
                    root: def_id,
                    instance: callee,
                });
            }
            worklist.push(callee);
        }
    }
}
