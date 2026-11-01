//! Root discovery on the complete monomorphized instance graph.

use no_alloc_report::parse_root_spec;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_middle::mono::{CollectionMode, MonoItem};
use rustc_middle::ty::{Instance, InstanceKind, TyCtxt};
use rustc_middle::util::Providers;
use rustc_span::Symbol;
use std::cell::Cell;
use std::collections::HashSet;
use std::sync::OnceLock;

fn root_attr_path() -> [Symbol; 2] {
    [Symbol::intern("no_alloc_tool"), Symbol::intern("root")]
}

pub fn is_root(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.tool_attrs(def_id, &root_attr_path()).next().is_some()
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

fn async_root_error(tcx: TyCtxt<'_>, def_id: DefId, selected: bool) -> String {
    let source = if selected { "requested" } else { "marked" };
    let path = root_path(tcx, def_id);
    format!("{source} root `{path}` is an async fn (only its future constructor would be checked)")
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
            // The local-definition pass below only sees local defs, so an
            // upstream async fn is reported here or not at all (it would be
            // `continue`d away with its `--root` spec already marked matched).
            // A local one is reported by both passes; the duplicate is
            // removed after sorting.
            selection_errors.push(async_root_error(tcx, def_id, selected));
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

    discover_upstream_shared(
        tcx,
        &mut seen_instances,
        &mut instantiated_defs,
        &mut roots,
        &mut selection_errors,
    );

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
            selection_errors.push(async_root_error(tcx, def_id, selected));
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
    selection_errors.dedup();
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

/// The collector's own `should_codegen_locally` hook, captured when
/// [`override_providers`] installs ours.
static COLLECTOR_SHOULD_CODEGEN_LOCALLY: OnceLock<
    for<'tcx> fn(TyCtxt<'tcx>, Instance<'tcx>) -> bool,
> = OnceLock::new();

thread_local! {
    /// Set only around [`unfiltered_used_items`], so every other query in the
    /// compilation sees the collector's real hook.
    static SEE_UPSTREAM_BODIES: Cell<bool> = const { Cell::new(false) };
}

/// Installs the `should_codegen_locally` wrapper that [`discover`] needs. Must
/// be called from the driver's `override_queries`.
pub fn override_providers(providers: &mut Providers) {
    // A second install would capture our own wrapper as "the original" and
    // recurse forever, so only the first one takes effect.
    if COLLECTOR_SHOULD_CODEGEN_LOCALLY
        .set(providers.hooks.should_codegen_locally)
        .is_ok()
    {
        providers.hooks.should_codegen_locally = should_codegen_locally;
    }
}

fn should_codegen_locally<'tcx>(tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
    let Some(original) = COLLECTOR_SHOULD_CODEGEN_LOCALLY.get() else {
        return true;
    };
    if original(tcx, instance) {
        return true;
    }
    if !SEE_UPSTREAM_BODIES.with(Cell::get) {
        return false;
    }
    // Upstream instantiations the collector links to rather than lists, but
    // whose MIR is available to walk. Foreign items, statics and bodies
    // without MIR stay excluded: `items_of_instance` has no MIR to read for
    // them.
    if instance.upstream_monomorphization(tcx).is_some() {
        return true;
    }
    match instance.def {
        InstanceKind::Item(def_id) => {
            matches!(
                tcx.def_kind(def_id),
                DefKind::Fn | DefKind::AssocFn | DefKind::Closure
            ) && !tcx.is_foreign_item(def_id)
                && tcx.is_mir_available(def_id)
        }
        _ => false,
    }
}

/// What the collector would use from `instance`'s body if it were walking it
/// locally. The `items_of_instance` query itself cannot answer that: it drops
/// every neighbour `should_codegen_locally` rejects, which is exactly the
/// upstream-shared instances this discovery is after. The provider is called
/// directly, not through the query, so the filtered answer the collector
/// already cached for local instances is neither reused nor overwritten.
fn unfiltered_used_items<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
) -> Result<Vec<MonoItem<'tcx>>, ()> {
    let provider = tcx.query_system.local_providers.items_of_instance;
    let previous = SEE_UPSTREAM_BODIES.with(|flag| flag.replace(true));
    let result = provider(tcx, (instance, CollectionMode::UsedItems));
    SEE_UPSTREAM_BODIES.with(|flag| flag.set(previous));
    // The collector only treats a normalization error as fatal for instances
    // it queried itself. An upstream instance it never walked can fail here
    // with the build still going, so the caller must surface the failure
    // rather than treat the body as empty (ADR 0003: reject, don't assume).
    match result {
        Ok((used, _mentioned)) => Ok(used.iter().map(|item| item.node).collect()),
        Err(_) => Err(()),
    }
}

/// A generic instantiation that an upstream crate already emitted is shared
/// (`-Zshare-generics`, on by default at opt-level 0 and always for
/// `#[inline(never)]`), so the local collector links to it instead of
/// listing it among the mono items, and never walks its body. A marked root
/// reached only through such a body would silently vanish from discovery.
/// This repeats the collector's walk (`items_of_instance`, so calls, drops,
/// reify and unsize casts and fn items in consts all count) from every local
/// mono item, enters each `Fn` neighbour that is not itself a local mono item,
/// and adopts any marked generic root it meets.
fn discover_upstream_shared<'tcx>(
    tcx: TyCtxt<'tcx>,
    seen_instances: &mut HashSet<Instance<'tcx>>,
    instantiated_defs: &mut HashSet<DefId>,
    roots: &mut Vec<DiscoveredRoot<'tcx>>,
    selection_errors: &mut Vec<String>,
) {
    let partitions = tcx.collect_and_partition_mono_items(());
    let local: HashSet<Instance<'tcx>> = partitions
        .codegen_units
        .iter()
        .flat_map(|cgu| cgu.items().keys())
        .filter_map(|item| match item {
            MonoItem::Fn(instance) => Some(*instance),
            _ => None,
        })
        .collect();
    let mut worklist: Vec<Instance<'tcx>> = local.iter().copied().collect();
    let mut visited: HashSet<Instance<'tcx>> = HashSet::new();
    while let Some(instance) = worklist.pop() {
        let Ok(items) = unfiltered_used_items(tcx, instance) else {
            selection_errors.push(format!(
                "could not collect the body of `{instance}` while discovering upstream-shared roots"
            ));
            continue;
        };
        for item in items {
            let MonoItem::Fn(callee) = item else {
                continue;
            };
            if local.contains(&callee) || !visited.insert(callee) {
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
                // The same async refusal as the other two discovery passes:
                // a non-workspace dependency can carry the raw marker on an
                // `async fn` (only the proc macro refuses it), and adopting
                // it would check just the future constructor. Not adopted,
                // but the walk still continues into it below: it costs
                // nothing and keeps this loop's invariant (every unshared
                // neighbour is visited) free of special cases.
                if matches!(tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn)
                    && tcx.asyncness(def_id).is_async()
                {
                    selection_errors.push(async_root_error(tcx, def_id, false));
                    worklist.push(callee);
                    continue;
                }
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
