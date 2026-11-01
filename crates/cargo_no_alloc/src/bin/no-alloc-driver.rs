//! `no-alloc-driver`: an out-of-tree `rustc_driver` binary substituted in as
//! cargo's compiler wrapper with real diagnostics (spans, `struct_span_err`
//! with one `span_note` per chain frame), a report fragment per compilation
//! (merged into `report.json` by `cargo-no-alloc`), `NO_ALLOC_WARN_ONLY`,
//! and errors fail the build. Skips codegen entirely via `Compilation::Stop`
//! on a hard error — there's no point codegening a crate whose check failed.
#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_span;

use no_alloc_analysis::roots::DiscoveredRoot;
use no_alloc_report::{Environment, PanicPaths, Report, ReportFragment};
use rustc_driver::{Callbacks, Compilation};
use rustc_interface::interface;
use rustc_middle::ty::TyCtxt;
use std::path::{Path, PathBuf};

/// Reads an `NO_ALLOC_*` boolean env var set by `cargo-no-alloc` itself
/// (`"1"`/`"0"`) — invocation-level flags like `--all-crates`/`--build-std`
/// aren't visible on `Session`, so they cross the process boundary as env
/// vars like everything else the driver needs from the wrapper. Only exactly
/// `"1"` is true, so an unset or garbled value can never turn a flag on.
fn env_flag(name: &str) -> bool {
    std::env::var(name).as_deref() == Ok("1")
}

/// `NO_ALLOC_STRICT`, the `--strict` mode (ADR 0008). Unset means the
/// default mode, so a direct `RUSTC=no-alloc-driver` build gets the
/// documented default. Unlike the other flags, any value other than `0`/`1`
/// is an error rather than "off": someone who wrote `NO_ALLOC_STRICT=true`
/// asked for the stronger guarantee, and silently giving them the weaker one
/// is the wrong direction to fail in.
fn parse_strict(value: Option<&str>) -> Result<PanicPaths, String> {
    match value {
        None | Some("0") => Ok(PanicPaths::OutOfScope),
        Some("1") => Ok(PanicPaths::Checked),
        Some(other) => Err(format!(
            "NO_ALLOC_STRICT must be `1` or `0` (or unset), not `{other}`"
        )),
    }
}

fn environment(tcx: TyCtxt<'_>, panic_paths: PanicPaths) -> Environment {
    no_alloc_analysis::boundary::environment(
        tcx,
        env_flag("NO_ALLOC_ALL_CRATES"),
        env_flag("NO_ALLOC_BUILD_STD"),
        panic_paths,
    )
}

/// Flags the driver needs on every invocation. `cargo-no-alloc` normally
/// supplies these via `CARGO_ENCODED_RUSTFLAGS`, but the driver re-adds any
/// that are missing so invoking it directly as `RUSTC=no-alloc-driver` also
/// works (see docs/design.md, Invocation). The list itself is the wrapper's,
/// so the two cannot drift.
const REQUIRED_FLAGS: &[&str] = cargo_no_alloc::REQUIRED_RUSTFLAGS;

struct NoAllocCallbacks {
    // Read once, before compiling, so a bad value fails the invocation up
    // front and the traversal and the recorded environment cannot disagree.
    panic_paths: PanicPaths,
}

impl Callbacks for NoAllocCallbacks {
    fn config(&mut self, config: &mut interface::Config) {
        config.override_queries = Some(|_, providers| {
            no_alloc_analysis::roots::override_providers(providers);
        });
    }

    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &interface::Compiler,
        tcx: TyCtxt<'tcx>,
    ) -> Compilation {
        let warn_only = env_flag("NO_ALLOC_WARN_ONLY");
        let panic_paths = self.panic_paths;
        let discovery = no_alloc_analysis::roots::discover(tcx);
        let mut report = Report {
            roots: Vec::new(),
            // Claimed below, and only if this compilation actually produced
            // verdicts: Cargo does not pass the target `RUSTFLAGS` to host
            // units (build scripts, proc macros), so a wrapped host unit
            // compiles under a different panic strategy than the code being
            // checked. Letting its empty fragment state one would make every
            // merge in a workspace with a build script disagree with itself
            // and drop the field entirely.
            panic_strategy: None,
            selection_errors: discovery.selection_errors,
            ..Report::default()
        };
        let mut any_hard_error = false;
        // Only a *checked instance* licenses a strategy claim. A
        // `NotInstantiated` root is a marker this compilation found and did
        // not check, which a wrapped host unit can have as easily as a
        // target one — and the host unit compiles under different panic
        // flags, so letting it claim a strategy reintroduces the
        // disagreement that empties the field.
        let mut checked_an_instance = false;

        for root in discovery.roots {
            match root {
                DiscoveredRoot::NotInstantiated { root } => {
                    report
                        .roots
                        .push(no_alloc_analysis::boundary::not_instantiated_verdict(
                            tcx, root,
                        ));
                }
                DiscoveredRoot::Instance { root, instance } => {
                    checked_an_instance = true;
                    let checked =
                        no_alloc_analysis::traversal::check_instance(tcx, instance, panic_paths);
                    if no_alloc_analysis::diagnostics::emit(tcx, &checked, warn_only) {
                        any_hard_error = true;
                    }
                    report.roots.push(no_alloc_analysis::boundary::root_verdict(
                        tcx,
                        root,
                        instance,
                        checked.verdict,
                        environment(tcx, panic_paths),
                    ));
                }
            }
        }

        if checked_an_instance {
            let panic_strategy = no_alloc_analysis::boundary::panic_strategy(tcx);
            report.panic_strategy = Some(panic_strategy);
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

/// Fails unless `injected` (argv[1], the rustc Cargo was asked to run) is the
/// `NO_ALLOC_EXPECTED_RUSTC` that `cargo-no-alloc` validated. Compared after
/// canonicalization so a symlinked spelling of the same file is accepted; a
/// missing variable fails closed, since then nothing vouches for the path.
fn verify_injected_rustc(injected: &str) -> Result<(), String> {
    let expected = std::env::var_os("NO_ALLOC_EXPECTED_RUSTC").ok_or_else(|| {
        format!(
            "cargo passed `{injected}` as rustc but NO_ALLOC_EXPECTED_RUSTC is unset; \
             run the driver through `cargo no-alloc`"
        )
    })?;
    let canonical = |path: &std::ffi::OsStr| {
        std::fs::canonicalize(path)
            .map_err(|error| format!("cannot resolve rustc path {path:?}: {error}"))
    };
    let (injected_real, expected_real) = (canonical(injected.as_ref())?, canonical(&expected)?);
    if injected_real == expected_real {
        Ok(())
    } else {
        Err(format!(
            "cargo invoked `{injected}` as rustc, but the checker validated {expected:?}; a \
             custom RUSTC / build.rustc cannot be honoured because the driver compiles with \
             its own rustc"
        ))
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
        // The driver compiles with the rustc it links, ignoring this path, so
        // accepting any `…/rustc` would let a substitute (a script adding
        // `--cfg` flags) shape the shipped build while the checked one runs
        // without it. `cargo-no-alloc` names the rustc it validated; anything
        // else is refused rather than silently replaced.
        if let Err(error) = verify_injected_rustc(&args[1]) {
            tracing::error!("{error}");
            eprintln!("error: no_alloc: {error}");
            return std::process::ExitCode::FAILURE;
        }
        args.remove(1);
    }

    for flag in REQUIRED_FLAGS {
        if !args.iter().any(|a| a == flag) {
            args.push((*flag).to_string());
        }
    }

    let strict = std::env::var_os("NO_ALLOC_STRICT");
    let panic_paths = match parse_strict(
        strict
            .as_ref()
            .map(|value| value.to_string_lossy())
            .as_deref(),
    ) {
        Ok(panic_paths) => panic_paths,
        Err(error) => {
            tracing::error!("{error}");
            eprintln!("error: no_alloc: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let mut callbacks = NoAllocCallbacks { panic_paths };
    rustc_driver::catch_with_exit_code(|| {
        rustc_driver::run_compiler(&args, &mut callbacks);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_is_exactly_zero_or_one_and_anything_else_is_an_error() {
        assert_eq!(parse_strict(None), Ok(PanicPaths::OutOfScope));
        assert_eq!(parse_strict(Some("0")), Ok(PanicPaths::OutOfScope));
        assert_eq!(parse_strict(Some("1")), Ok(PanicPaths::Checked));
        for value in ["true", "yes", "", " 1", "01", "2"] {
            assert!(parse_strict(Some(value)).is_err(), "`{value}`");
        }
    }
}
