//! `no-alloc-driver`: an out-of-tree `rustc_driver` binary substituted in as
//! cargo's compiler wrapper with real diagnostics (spans, `struct_span_err`
//! with one `span_note` per chain frame), `report.json`, `NO_ALLOC_WARN_ONLY`,
//! and errors fail the build. Skips codegen entirely via `Compilation::Stop`
//! on a hard error — there's no point codegening a crate whose check failed.
#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_span;

use no_alloc_analysis::roots::DiscoveredRoot;
use no_alloc_report::{Report, ReportFragment, RootVerdict};
use rustc_driver::{Callbacks, Compilation};
use rustc_interface::interface;
use rustc_middle::ty::TyCtxt;
use std::path::{Path, PathBuf};

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
        let warn_only = std::env::var("NO_ALLOC_WARN_ONLY").as_deref() == Ok("1");
        let discovery = no_alloc_analysis::roots::discover(tcx);
        let mut report = Report {
            roots: Vec::new(),
            selection_errors: discovery.selection_errors,
        };
        let mut any_hard_error = false;

        for root in discovery.roots {
            match root {
                DiscoveredRoot::NotInstantiated { root } => {
                    let root_path = no_alloc_analysis::roots::root_path(tcx, root);
                    report.roots.push(RootVerdict {
                        root: root_path.clone(),
                        instance: root_path,
                        verdict: no_alloc_report::Verdict::NotInstantiated,
                    });
                }
                DiscoveredRoot::Instance { root, instance } => {
                    let root_path = no_alloc_analysis::roots::root_path(tcx, root);
                    let checked = no_alloc_analysis::traversal::check_instance(tcx, instance);
                    if no_alloc_analysis::diagnostics::emit(tcx, &checked, warn_only) {
                        any_hard_error = true;
                    }
                    report.roots.push(RootVerdict {
                        root: root_path,
                        instance: instance.to_string(),
                        verdict: checked.verdict,
                    });
                }
            }
        }

        let fragment_dir = std::env::var_os("NO_ALLOC_FRAGMENT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target/no-alloc/fragments"));
        let crate_name = tcx.crate_name(rustc_span::def_id::LOCAL_CRATE);
        // PID alone can collide across separate driver invocations if the OS
        // recycles it during a long build; the nanosecond timestamp makes
        // that astronomically unlikely without adding a dependency.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let fragment_path =
            fragment_dir.join(format!("{crate_name}-{}-{nanos}.json", std::process::id()));
        let fragment = ReportFragment {
            report,
            matched_root_specs: discovery.matched_root_specs,
        };
        if let Err(error) = fragment.write_to_file(&fragment_path) {
            tcx.dcx().err(format!(
                "no_alloc: failed to write report fragment: {error}"
            ));
            any_hard_error = true;
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
                .unwrap_or_else(|_| "off".into()),
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

    let mut callbacks = NoAllocCallbacks;
    rustc_driver::catch_with_exit_code(|| {
        rustc_driver::run_compiler(&args, &mut callbacks);
    })
}
