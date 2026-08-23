//! Root discovery on the complete monomorphized instance graph.

use no_alloc_report::parse_root_spec;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_middle::mono::MonoItem;
use rustc_middle::ty::{Instance, TyCtxt};
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
        if (selected || is_root(tcx, def_id)) && seen_instances.insert(*instance) {
            instantiated_defs.insert(def_id);
            roots.push(DiscoveredRoot::Instance {
                root: def_id,
                instance: *instance,
            });
        }
    }

    // Mono collection cannot contain uncalled local functions. Seed those
    // explicitly, while preserving NotInstantiated for generic definitions.
    for local in tcx.iter_local_def_id() {
        let def_id = local.to_def_id();
        let path = root_path(tcx, def_id);
        let selected = record_matches(tcx, def_id, &requested, &mut matched);
        if selected && !matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn) {
            selection_errors.push(format!("requested root `{path}` is not a function"));
            continue;
        }
        if !(selected || is_root(tcx, def_id)) || instantiated_defs.contains(&def_id) {
            continue;
        }
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
