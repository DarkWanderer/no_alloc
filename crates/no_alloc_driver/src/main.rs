//! `no-alloc-driver`: an out-of-tree `rustc_driver` binary substituted in as
//! cargo's compiler wrapper. M5: real diagnostics (spans, `struct_span_err`
//! with one `span_note` per chain frame), `report.json`, `NO_ALLOC_WARN_ONLY`,
//! and errors fail the build. Skips codegen entirely via `Compilation::Stop`
//! on a hard error — there's no point codegening a crate whose check failed.
#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_span;

use no_alloc_analysis::roots::RootInstances;
use no_alloc_report::{Report, RootVerdict};
use rustc_driver::{Callbacks, Compilation};
use rustc_interface::interface;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::LOCAL_CRATE;
use std::collections::HashSet;
use std::path::Path;
use tracing::info;

const REPORT_PATH: &str = "target/no-alloc/report.json";

/// Flags the driver needs on every invocation. `cargo-no-alloc` normally
/// supplies these via `CARGO_ENCODED_RUSTFLAGS`, but the driver re-adds any
/// that are missing so invoking it directly as `RUSTC=no-alloc-driver` also
/// works (see docs/design.md, Invocation).
const REQUIRED_FLAGS: &[&str] = &[
    "--cfg=no_alloc_check",
    "--check-cfg=cfg(no_alloc_check)",
    "-Zcrate-attr=feature(register_tool)",
    "-Zcrate-attr=register_tool(no_alloc_tool)",
    "-Zalways-encode-mir",
];

struct NoAllocCallbacks;

impl Callbacks for NoAllocCallbacks {
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &interface::Compiler,
        tcx: TyCtxt<'tcx>,
    ) -> Compilation {
        info!(crate_name = %tcx.crate_name(LOCAL_CRATE), "after_analysis");

        no_alloc_analysis::mono_dump::dump_mono_items(tcx);
        no_alloc_analysis::roots::probe_foreign_roots(tcx);

        let warn_only = std::env::var("NO_ALLOC_WARN_ONLY").as_deref() == Ok("1");

        let roots: HashSet<_> = no_alloc_analysis::roots::local_roots(tcx)
            .into_iter()
            .chain(no_alloc_analysis::roots::env_roots(tcx))
            .collect();

        let mut report = Report::default();
        let mut any_hard_error = false;

        for def_id in roots {
            let def_id = def_id.to_def_id();
            let root_path = tcx.def_path_str(def_id);
            info!(def_path = %root_path, "root");

            match no_alloc_analysis::roots::instances_for_root(tcx, def_id) {
                RootInstances::NotInstantiated => {
                    info!(def_path = %root_path, "root not instantiated in this crate");
                    report.roots.push(RootVerdict {
                        root: root_path.clone(),
                        instance: root_path,
                        verdict: no_alloc_report::Verdict::NotInstantiated,
                    });
                }
                RootInstances::Instances(instances) => {
                    for instance in instances {
                        let checked = no_alloc_analysis::traversal::check_instance(tcx, instance);
                        info!(instance = %instance, verdict = ?checked.verdict, "checked");

                        if no_alloc_analysis::diagnostics::emit(tcx, &checked, warn_only) {
                            any_hard_error = true;
                        }

                        report.roots.push(RootVerdict {
                            root: root_path.clone(),
                            instance: instance.to_string(),
                            verdict: checked.verdict,
                        });
                    }
                }
            }
        }

        if !report.roots.is_empty() {
            if let Err(err) = report.write_to_file(std::path::Path::new(REPORT_PATH)) {
                tracing::error!(%err, path = REPORT_PATH, "failed to write report.json");
            }
        }

        if any_hard_error {
            Compilation::Stop
        } else {
            Compilation::Continue
        }
    }
}

fn main() -> std::process::ExitCode {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("NO_ALLOC_LOG")
                .unwrap_or_else(|_| "no_alloc_driver=info".into()),
        )
        .init();

    let mut args: Vec<String> = std::env::args().collect();

    if args.len() <= 1 {
        tracing::error!("missing rustc invocation");
        return std::process::ExitCode::FAILURE;
    }

    // Under `RUSTC_WORKSPACE_WRAPPER`, cargo invokes us as
    // `no-alloc-driver <real-rustc> <rustc args...>`; strip the injected
    // real-rustc path so `run_compiler` sees a normal rustc argv. When
    // invoked directly as `RUSTC=no-alloc-driver` there is no such element.
    if Path::new(&args[1]).file_stem().and_then(|s| s.to_str()) == Some("rustc") {
        args.remove(1);
    }

    for flag in REQUIRED_FLAGS {
        if !args.iter().any(|a| a == flag) {
            args.push((*flag).to_string());
        }
    }

    info!(argc = args.len(), args = ?args, "invoking rustc");

    let mut callbacks = NoAllocCallbacks;
    rustc_driver::catch_with_exit_code(|| {
        rustc_driver::run_compiler(&args, &mut callbacks);
    })
}
