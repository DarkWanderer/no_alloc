//! M3 verification-only walker. Proves `leaf::allocates` actually fires on
//! a real `Box::new` chain, and measures std MIR coverage on the stock
//! sysroot, **before** either fact is trusted inside the production
//! traversal (M4). This is deliberately not that traversal: no Drop edges,
//! no reject-vs-continue distinction for `dyn`/fn-pointer callees (they're
//! just not followed), no violation/report types — a plain BFS over
//! `Call`/`TailCall` edges that stops at the first allocator terminal it
//! finds and prints the chain plus a MIR-availability tally.

use crate::leaf::allocates;
use rustc_middle::mir::TerminatorKind;
use rustc_middle::ty::{self, Instance, TyCtxt};
use std::collections::{HashMap, HashSet, VecDeque};
use tracing::info;

pub fn debug_probe_chain<'tcx>(tcx: TyCtxt<'tcx>, root: Instance<'tcx>) {
    let typing_env = ty::TypingEnv::fully_monomorphized();
    let mut visited: HashSet<Instance<'tcx>> = HashSet::new();
    let mut parent: HashMap<Instance<'tcx>, Instance<'tcx>> = HashMap::new();
    let mut queue: VecDeque<Instance<'tcx>> = VecDeque::new();
    let mut mir_available = 0usize;
    let mut mir_missing = 0usize;

    visited.insert(root);
    queue.push_back(root);

    while let Some(instance) = queue.pop_front() {
        let has_mir = tcx.is_mir_available(instance.def_id());
        if has_mir {
            mir_available += 1;
        } else {
            mir_missing += 1;
        }

        if allocates(tcx, instance) {
            report_chain(tcx, &parent, instance, mir_available, mir_missing);
            return;
        }
        if !has_mir {
            continue;
        }

        let body = tcx.instance_mir(instance.def);
        for bb in body.basic_blocks.iter() {
            let (TerminatorKind::Call { func, .. } | TerminatorKind::TailCall { func, .. }) =
                &bb.terminator().kind
            else {
                continue;
            };
            let callee_ty = func.ty(body, tcx);
            let callee_ty = instance.instantiate_mir_and_normalize_erasing_regions(
                tcx,
                typing_env,
                ty::EarlyBinder::bind(tcx, callee_ty),
            );
            let ty::FnDef(def_id, args) = callee_ty.kind() else {
                // fn pointer / dyn dispatch: M4's traversal rejects these;
                // this probe just doesn't follow them.
                continue;
            };
            // `args` carries a `Binder` here (higher-ranked signatures); a
            // callee with unresolved bound vars isn't statically callable
            // either, so this probe just doesn't follow it.
            let Some(args) = args.no_bound_vars() else {
                continue;
            };
            let Ok(Some(callee)) = Instance::try_resolve(tcx, typing_env, *def_id, args) else {
                continue;
            };
            if visited.insert(callee) {
                parent.insert(callee, instance);
                queue.push_back(callee);
            }
        }
    }

    info!(
        mir_available,
        mir_missing, "probe_chain: no allocator terminal reachable from root (search exhausted)"
    );
}

fn report_chain<'tcx>(
    tcx: TyCtxt<'tcx>,
    parent: &HashMap<Instance<'tcx>, Instance<'tcx>>,
    terminal: Instance<'tcx>,
    mir_available: usize,
    mir_missing: usize,
) {
    let mut chain = vec![terminal];
    let mut cur = terminal;
    while let Some(&p) = parent.get(&cur) {
        chain.push(p);
        cur = p;
    }
    chain.reverse();

    for (depth, instance) in chain.iter().enumerate() {
        info!(
            depth,
            instance = %instance,
            has_mir = tcx.is_mir_available(instance.def_id()),
            "probe_chain frame"
        );
    }
    info!(
        terminal = %terminal,
        chain_len = chain.len(),
        mir_available,
        mir_missing,
        "probe_chain terminated: allocator"
    );
}
