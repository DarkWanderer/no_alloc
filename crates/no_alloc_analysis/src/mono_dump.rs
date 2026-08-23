//! M2 observable: enumerate every `MonoItem::Fn` instance across all
//! codegen units and print it with concrete substitutions. This is the mono
//! graph the rest of the analysis will walk from M4 onward.

use rustc_middle::mono::MonoItem;
use rustc_middle::ty::TyCtxt;
use tracing::info;

pub fn dump_mono_items(tcx: TyCtxt<'_>) {
    let partitions = tcx.collect_and_partition_mono_items(());
    let mut fn_count = 0usize;

    for cgu in partitions.codegen_units {
        for (item, _data) in cgu.items() {
            let MonoItem::Fn(instance) = item else {
                continue;
            };
            fn_count += 1;
            info!(cgu = %cgu.name(), instance = %instance, "mono_item_fn");
        }
    }

    info!(
        codegen_units = partitions.codegen_units.len(),
        fn_mono_items = fn_count,
        "mono graph dump complete"
    );
}
