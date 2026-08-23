//! Root collection via the `no_alloc_tool::root` tool attribute (see
//! ADR 0002). `get_attrs_by_path` dispatches to HIR for local `DefId`s and
//! to `attrs_for_def`'s `separate_provide_extern` provider for foreign
//! ones, so the same call answers both "what are this crate's roots" and
//! the M2 cross-crate-metadata probe below.

use no_alloc_report::parse_root_spec;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_middle::mono::MonoItem;
use rustc_middle::ty::{Instance, TyCtxt};
use rustc_span::Symbol;
use std::collections::HashSet;
use std::env;
use tracing::info;

fn root_attr_path() -> [Symbol; 2] {
    [Symbol::intern("no_alloc_tool"), Symbol::intern("root")]
}

pub fn is_root(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.get_attrs_by_path(def_id, &root_attr_path())
        .next()
        .is_some()
}

/// Local defs in this crate carrying `#[no_alloc_tool::root]`.
pub fn local_roots(tcx: TyCtxt<'_>) -> Vec<LocalDefId> {
    tcx.iter_local_def_id()
        .filter(|&def_id| is_root(tcx, def_id.to_def_id()))
        .collect()
}

/// `NO_ALLOC_ROOTS` fallback: comma-separated `def_path_str`s, matched
/// exactly (never a prefix/substring — see `no_alloc_report::parse_root_spec`).
/// Unblocks third-party code the attribute can't reach.
pub fn env_roots(tcx: TyCtxt<'_>) -> Vec<LocalDefId> {
    let Ok(spec) = env::var("NO_ALLOC_ROOTS") else {
        return Vec::new();
    };
    let wanted: HashSet<String> = parse_root_spec(&spec).into_iter().collect();
    if wanted.is_empty() {
        return Vec::new();
    }
    tcx.iter_local_def_id()
        .filter(|&def_id| wanted.contains(&tcx.def_path_str(def_id.to_def_id())))
        .collect()
}

pub enum RootInstances<'tcx> {
    Instances(Vec<Instance<'tcx>>),
    /// A generic root with no instantiation in this crate — info, not a
    /// failure: there's no MIR to walk for a signature no one called.
    NotInstantiated,
}

/// A root `DefId`'s instances, mono-site: a generic root gets one verdict
/// per instantiation actually present in this crate's mono graph (the case
/// that justifies mono-site analysis at all — two instantiations of the
/// same root can have different verdicts). A local non-generic root that
/// was never called still gets seeded via `Instance::mono` so it's checked
/// rather than silently skipped.
pub fn instances_for_root<'tcx>(tcx: TyCtxt<'tcx>, root: DefId) -> RootInstances<'tcx> {
    let partitions = tcx.collect_and_partition_mono_items(());
    let found: Vec<Instance<'tcx>> = partitions
        .codegen_units
        .iter()
        .flat_map(|cgu| cgu.items().keys())
        .filter_map(|item| match item {
            MonoItem::Fn(instance) if instance.def_id() == root => Some(*instance),
            _ => None,
        })
        .collect();

    if !found.is_empty() {
        return RootInstances::Instances(found);
    }

    if tcx.generics_of(root).requires_monomorphization(tcx) {
        RootInstances::NotInstantiated
    } else {
        RootInstances::Instances(vec![Instance::mono(tcx, root)])
    }
}

/// M2 probe: does the root attribute survive into a *dependency* crate's
/// metadata, i.e. can this crate see roots marked in crates it depends on
/// without the sidecar index? Walks each external crate's root module
/// (one level deep — sufficient for the M2 toy fixture) and logs any
/// `DefKind::Fn` child carrying the attribute.
pub fn probe_foreign_roots(tcx: TyCtxt<'_>) -> usize {
    let mut found = 0usize;
    for &cnum in tcx.crates(()) {
        let root = cnum.as_def_id();
        for child in tcx.module_children(root) {
            let Res::Def(DefKind::Fn, def_id) = child.res else {
                continue;
            };
            if is_root(tcx, def_id) {
                found += 1;
                info!(
                    crate_name = %tcx.crate_name(cnum),
                    def_path = %tcx.def_path_str(def_id),
                    "foreign root attribute visible via cross-crate metadata"
                );
            }
        }
    }
    info!(
        foreign_roots_found = found,
        "cross-crate root attribute probe complete"
    );
    found
}
