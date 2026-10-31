//! Turning fragments into a report, tainting it when the run cannot be
//! trusted, and saying so on stderr.

use crate::cli::StdMode;
use anyhow::{bail, ensure, Context, Result};
use no_alloc_report::{PanicStrategy, Report, ReportFragment, Verdict};
use std::collections::{BTreeSet, HashSet};
use std::fs::{self, File};
use std::path::Path;
use std::process::ExitStatus;

// Printed unconditionally (not just under --warn-only) because a clean exit
// with zero roots checked is not a passing result, just an unverified one —
// see the README's "By default only workspace members are instrumented".
// Also the message when roots were found but none was instantiated: those
// report `not_instantiated`, which is success without a verdict.
pub(crate) const ZERO_ROOTS_WARNING: &str = "no_alloc checked 0 root instances \
— nothing was analyzed. This usually means either no `#[no_alloc]` marker \
was reached while building, every marked generic function was never \
instantiated (reported `not_instantiated`), or the marker lives in a crate \
outside the workspace (pass --all-crates to instrument it).";

/// Records why the run is being refused in `report.json`, then returns the
/// error to bail with. The stale report is already gone by the time any
/// refusal can happen, so without this a consumer reading the file would find
/// either nothing or, worse, nothing distinguishing "refused" from "never
/// ran"; the persisted report is what keeps every refusal path consistent
/// with the configuration refusals.
pub(crate) fn refuse(report_path: &Path, reason: String) -> anyhow::Error {
    match rejected_report(vec![reason.clone()]).write_to_file(report_path) {
        Ok(()) => anyhow::anyhow!(reason),
        Err(error) => anyhow::Error::new(error).context(format!(
            "failed to record the refusal in report.json: {reason}"
        )),
    }
}

/// Whether the build the driver just ran mixes an immediate-abort crate
/// with a standard library that was not rebuilt to match.
///
/// What makes an observed `ImmediateAbort` strategy safe is that the
/// sysroot was rebuilt *in this build* — which is what `--build-std` does:
/// Cargo applies the same profile and the same `RUSTFLAGS` to those units
/// too. So `--build-std` with the strategy set by hand (the manifest, a
/// `config.toml`, `--profile`) is coherent and not flagged; only an
/// immediate-abort crate built against the untouched precompiled sysroot
/// is a mix worth rejecting.
///
/// `any_immediate_abort` comes from scanning the raw fragments
/// (`aggregate`), not from `report.panic_strategy`: the merged field goes to
/// `None` on any disagreement between fragments, which would silently hide
/// exactly the mix this check exists to catch (a target-specific config
/// selecting the strategy for the checked crate while a wrapped host unit
/// compiles under something else).
pub(crate) fn mixed_sysroot_panic_strategy(std_mode: StdMode, any_immediate_abort: bool) -> bool {
    !std_mode.rebuilds_std() && any_immediate_abort
}

/// Marks a report so that `Report::is_success()` says `false` on it, the
/// instant a mixed sysroot is detected — before the report is written to
/// disk. Without this, `report.json` could show every root passing (a
/// trivial function has no reason to care what std's panic strategy is)
/// while the process that produced it exits nonzero: a tool reading the
/// file directly, rather than this process's own exit code, would see a
/// false success. `selection_errors` is the existing channel for "this
/// report should not be trusted as-is" (already used for an unmatched
/// `--root`), so this reuses it rather than adding a second one.
pub(crate) fn taint_report_for_mixed_sysroot(report: &mut Report, mixed_sysroot: bool) {
    if mixed_sysroot {
        report.selection_errors.push(
            "this build compiled under `-Cpanic=immediate-abort` without rebuilding the \
             standard library (`--build-std`); every verdict above was produced against a \
             sysroot whose panic runtime does not match the strategy claimed"
                .to_owned(),
        );
    }
}

/// A report that records why the run was refused, so `is_success()` is
/// false for anyone reading the persisted file.
pub(crate) fn rejected_report(mut reasons: Vec<String>) -> Report {
    reasons.sort();
    reasons.dedup();
    Report {
        selection_errors: reasons,
        ..Report::default()
    }
}

/// A Cargo failure can stop the build before every wrapped unit was
/// compiled, so the fragments on disk may cover only part of it. When
/// nothing else already marks the report as failing, say so in the
/// persisted file — not only in this process's exit code — so a consumer
/// reading `report.json` cannot take a partial run for a passing one.
pub(crate) fn taint_report_for_failed_build(report: &mut Report, cargo_succeeded: bool) {
    if !cargo_succeeded && report.is_success() {
        report.selection_errors.push(
            "Cargo did not complete the build, so some roots may never have been checked"
                .to_owned(),
        );
    }
}

/// What [`aggregate`] learned from the fragments.
pub(crate) struct Aggregated {
    pub(crate) report: Report,
    /// Whether any verdict-bearing fragment observed `ImmediateAbort`,
    /// answered from the raw fragments because the merged report cannot say.
    pub(crate) any_immediate_abort: bool,
}

/// `Report::merge`'s agreed-strategy answer is the right thing for
/// `report.json` to state (ADR 0006) — it is silent rather than guessing
/// when fragments disagree. It is the wrong question for this wrapper's own
/// safety net, though: a target-specific `.cargo/config.toml` can compile
/// the checked crate under `ImmediateAbort` while a wrapped host unit (a
/// build script with a checked root of its own) compiles under `Unwind`,
/// and the disagreement collapses the merged strategy to `None` — silently
/// clearing the one signal `mixed_sysroot_panic_strategy` needs to catch
/// exactly that case. So this is answered directly from the fragments,
/// before merging can lose it: "did *any* verdict-bearing fragment observe
/// `ImmediateAbort`", regardless of what the others said.
pub(crate) fn aggregate(fragment_dir: &Path, requested: &BTreeSet<String>) -> Result<Aggregated> {
    let mut reports = Vec::new();
    let mut matched = HashSet::new();
    if fragment_dir.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(fragment_dir)?.collect::<std::io::Result<_>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let fragment: ReportFragment = serde_json::from_reader(File::open(entry.path())?)
                .with_context(|| format!("failed to parse {}", entry.path().display()))?;
            matched.extend(fragment.matched_root_specs);
            reports.push(fragment.report);
        }
    }
    let any_immediate_abort = reports.iter().any(|report| {
        report.checked_an_instance() && report.panic_strategy == Some(PanicStrategy::ImmediateAbort)
    });
    let mut report = Report::merge(reports);
    for root in requested {
        if !matched.contains(root) {
            report.selection_errors.push(format!(
                "requested root `{root}` did not match any item in the build"
            ));
        }
    }
    report.selection_errors.sort();
    report.selection_errors.dedup();
    Ok(Aggregated {
        report,
        any_immediate_abort,
    })
}

/// Refuses the run if configuration reported reasons it cannot be checked
/// faithfully: records them in `report.json` (not only on stderr, the same
/// way a mixed sysroot is) and bails before anything is built.
pub(crate) fn refuse_configuration(report_path: &Path, rejections: Vec<String>) -> Result<()> {
    if rejections.is_empty() {
        return Ok(());
    }
    let report = rejected_report(rejections);
    report
        .write_to_file(report_path)
        .context("failed to write final report.json")?;
    for error in &report.selection_errors {
        eprintln!("error: {error}");
    }
    bail!("this build's configuration cannot be checked faithfully; nothing was built");
}

/// Everything after the build: taint, record, report, and decide the exit.
///
/// The order is load-bearing. `taint_report_for_failed_build` only adds its
/// line if the report still `is_success()`, so it must run after the mixed
/// sysroot taint; both go in before the write, so the file and the exit
/// status agree; and the write precedes every `bail` so the evidence survives
/// a failure, `--warn-only` or not.
pub(crate) fn finalize(
    mut report: Report,
    report_path: &Path,
    cargo_status: ExitStatus,
    mixed_sysroot: bool,
    warn_only: bool,
) -> Result<()> {
    // The pre-flight guard reads the environment, and the strategy can also
    // arrive from the manifest (`cargo-features = ["panic-immediate-abort"]`
    // plus a profile), `config.toml`, or `--profile`. The mixed-sysroot check
    // asks the build itself: the driver recorded the strategy rustc actually
    // used, so a mixed configuration is caught however it was selected.
    //
    // Recorded into `selection_errors` before the report is written, not
    // just raised as a hard error afterward: `Report::is_success()` would
    // otherwise say `true` for a persisted `report.json` whose roots all
    // passed, even though those verdicts were produced against a sysroot
    // that was never rebuilt to match. A tool reading the file directly
    // (rather than this process's own exit code) needs the taint to be
    // in the artifact, not only on stderr.
    taint_report_for_mixed_sysroot(&mut report, mixed_sysroot);
    taint_report_for_failed_build(&mut report, cargo_status.success());
    // Persisted like the taints above, but as a warning: the exit status
    // stays 0 (a workspace may legitimately have no markers), yet a reader of
    // `report.json` alone must not take `roots: []`, or a list of roots that
    // were never instantiated, for a verified pass.
    if !report.checked_an_instance() {
        report.warnings.push(ZERO_ROOTS_WARNING.to_owned());
    }
    report
        .write_to_file(report_path)
        .context("failed to write final report.json")?;
    print_summary(&report, warn_only);
    // The report is written first (above), so the evidence survives this
    // failure regardless of `--warn-only` — a mixed sysroot invalidates the
    // very mechanism `--warn-only` relies on to interpret results, so unlike
    // an ordinary finding it is never downgraded to a warning.
    ensure!(
        !mixed_sysroot,
        "this build compiled with `-Cpanic=immediate-abort` against the precompiled standard \
         library, which carries its own panic runtime — so the report's verdicts would claim \
         panic paths that lower to `abort` when std's do not. Pass `--immediate-abort`, which \
         applies the strategy to both halves, or `--build-std` if you are setting the \
         strategy yourself (see docs/iterators.md)"
    );
    if !cargo_status.success() {
        bail!("Cargo exited with {cargo_status}");
    }
    if !warn_only && !report.is_success() {
        bail!("no_alloc check failed");
    }
    Ok(())
}

/// What the user sees on stderr once the report is on disk: each selection
/// error, the count line, and the zero-roots warning.
pub(crate) fn print_summary(report: &Report, warn_only: bool) {
    for error in &report.selection_errors {
        eprintln!("{}: {error}", if warn_only { "warning" } else { "error" });
    }
    let failures = report
        .roots
        .iter()
        .filter(|root| !matches!(root.verdict, Verdict::Pass | Verdict::NotInstantiated))
        .count();
    // Counts instances with a verdict so this line agrees with the
    // "checked 0 root instances" warning for a not-instantiated-only run.
    let checked = report
        .roots
        .iter()
        .filter(|root| !matches!(root.verdict, Verdict::NotInstantiated))
        .count();
    eprintln!("no_alloc: checked {checked} root instance(s), {failures} finding(s)");
    // No verdict at all (0 roots, or only `not_instantiated` ones) is a
    // silent-pass hazard, not a clean pass: it exits 0 and looks identical to
    // "everything checked out" unless called out here.
    if !report.checked_an_instance() {
        eprintln!("warning: {ZERO_ROOTS_WARNING}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use no_alloc_report::{Environment, RootVerdict};

    #[test]
    fn aggregate_detects_immediate_abort_even_when_fragments_disagree() -> Result<()> {
        // The P1 scenario: a target-specific config selects immediate-abort
        // for the checked crate, while a wrapped host unit (a build script
        // with a checked root of its own) compiles under something else.
        // `Report::merge` correctly calls that disagreement `None` for
        // `report.json`'s own sake — but `aggregate`'s second return value
        // must still see the `ImmediateAbort` fragment, or the safety net in
        // `run` never fires.
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_aggregate_conflict_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&directory)?;
        let root = |name: &str, panic_strategy| RootVerdict {
            root: name.to_owned(),
            instance: name.to_owned(),
            verdict: Verdict::Pass,
            environment: Some(Environment {
                panic_strategy,
                opt_level: "No".into(),
                mir_opt_level: 1,
                overflow_checks: false,
                debug_assertions: false,
                target_triple: "x86_64-unknown-linux-gnu".into(),
                rustc_version: "1.99.0-nightly".into(),
                all_crates: false,
                build_std: true,
            }),
        };
        ReportFragment {
            report: Report {
                roots: vec![root("target::checked", PanicStrategy::ImmediateAbort)],
                panic_strategy: Some(PanicStrategy::ImmediateAbort),
                ..Report::default()
            },
            matched_root_specs: vec![],
        }
        .write_to_file(&directory.join("target.json"))?;
        ReportFragment {
            report: Report {
                roots: vec![root("build_script::checked", PanicStrategy::Unwind)],
                panic_strategy: Some(PanicStrategy::Unwind),
                ..Report::default()
            },
            matched_root_specs: vec![],
        }
        .write_to_file(&directory.join("host.json"))?;

        let Aggregated {
            report,
            any_immediate_abort,
        } = aggregate(&directory, &BTreeSet::new())?;
        // The report itself is silent about the disagreement, as designed...
        assert_eq!(report.panic_strategy, None);
        assert_eq!(report.roots.len(), 2);
        assert_ne!(report.roots[0].environment, report.roots[1].environment);
        assert!(report.selection_errors.is_empty());
        // ...but the safety-net signal still catches the mix.
        assert!(any_immediate_abort);
        assert!(mixed_sysroot_panic_strategy(
            StdMode::Precompiled,
            any_immediate_abort
        ));

        std::fs::remove_dir_all(directory)?;
        Ok(())
    }

    #[test]
    fn taint_report_for_mixed_sysroot_flips_is_success() {
        // The scenario this exists for: a trivial root that would pass
        // under any panic strategy, so nothing about the traversal itself
        // objects — but the sysroot claim is untrustworthy, and a reader of
        // `report.json` alone (not this process's exit code) needs that to
        // show up in the file.
        let passing = Report {
            roots: vec![RootVerdict {
                root: "a".into(),
                instance: "a".into(),
                verdict: Verdict::Pass,
                environment: Some(Environment {
                    panic_strategy: PanicStrategy::ImmediateAbort,
                    opt_level: "No".into(),
                    mir_opt_level: 1,
                    overflow_checks: false,
                    debug_assertions: false,
                    target_triple: "x86_64-unknown-linux-gnu".into(),
                    rustc_version: "1.99.0-nightly".into(),
                    all_crates: false,
                    build_std: true,
                }),
            }],
            panic_strategy: Some(PanicStrategy::ImmediateAbort),
            ..Report::default()
        };
        assert!(passing.is_success());

        let mut tainted = passing.clone();
        taint_report_for_mixed_sysroot(&mut tainted, true);
        assert!(!tainted.is_success());
        assert_eq!(tainted.selection_errors.len(), 1);

        // Not tainted when the sysroot isn't mixed: a coherent report is
        // left exactly as `aggregate` produced it.
        let mut untouched = passing.clone();
        taint_report_for_mixed_sysroot(&mut untouched, false);
        assert_eq!(untouched, passing);
    }

    #[test]
    fn mixed_sysroot_is_flagged_only_without_build_std() {
        // The dangerous case: crate compiled immediate-abort, sysroot left
        // precompiled (manifest/config selected the strategy, `--build-std`
        // was not passed).
        assert!(mixed_sysroot_panic_strategy(StdMode::Precompiled, true));
        // `--build-std` rebuilds the sysroot under the same flags, so the
        // same observation is coherent once it is set.
        assert!(!mixed_sysroot_panic_strategy(StdMode::BuildStd, true));
        assert!(!mixed_sysroot_panic_strategy(StdMode::ImmediateAbort, true));
        // Nothing observed has nothing to be mixed with.
        assert!(!mixed_sysroot_panic_strategy(StdMode::Precompiled, false));
    }

    #[test]
    fn zero_roots_warning_names_the_likely_causes() {
        assert!(ZERO_ROOTS_WARNING.contains("--all-crates"));
        assert!(ZERO_ROOTS_WARNING.contains("not_instantiated"));
        assert!(ZERO_ROOTS_WARNING.contains("0 root instances"));
        // Printed with a `warning:` prefix and persisted without one.
        assert!(!ZERO_ROOTS_WARNING.starts_with("warning:"));
    }

    #[test]
    fn aggregate_ignores_non_json_and_reports_unmatched() -> Result<()> {
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_aggregate_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&directory)?;
        std::fs::write(directory.join("ignore.txt"), "not json")?;
        ReportFragment {
            report: Report::default(),
            matched_root_specs: vec!["matched::root".into()],
        }
        .write_to_file(&directory.join("fragment.json"))?;

        let Aggregated {
            report,
            any_immediate_abort,
        } = aggregate(
            &directory,
            &["matched::root".to_owned(), "missing::root".to_owned()]
                .into_iter()
                .collect(),
        )?;
        assert_eq!(report.selection_errors.len(), 1);
        assert!(report.selection_errors[0].contains("missing::root"));
        assert!(!any_immediate_abort);
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }

    #[test]
    fn aggregate_accepts_missing_fragment_directory() -> Result<()> {
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_missing_{}_{}",
            std::process::id(),
            line!()
        ));
        let Aggregated {
            report,
            any_immediate_abort,
        } = aggregate(&directory, &BTreeSet::new())?;
        assert_eq!(report, Report::default());
        assert!(!any_immediate_abort);
        Ok(())
    }

    #[test]
    fn failed_build_taints_an_otherwise_passing_report() {
        let mut report = Report::default();
        assert!(report.is_success());
        taint_report_for_failed_build(&mut report, true);
        assert!(report.is_success());
        taint_report_for_failed_build(&mut report, false);
        assert!(!report.is_success());
    }

    #[test]
    fn failed_build_does_not_add_noise_to_an_already_failing_report() {
        let mut report = Report {
            selection_errors: vec!["already failing".into()],
            ..Report::default()
        };
        taint_report_for_failed_build(&mut report, false);
        assert_eq!(report.selection_errors, vec!["already failing".to_owned()]);
    }

    #[test]
    fn a_refused_run_leaves_a_report_that_is_not_a_success() {
        let report = rejected_report(vec!["b".to_owned(), "a".to_owned(), "a".to_owned()]);
        assert!(!report.is_success());
        assert_eq!(report.selection_errors, ["a", "b"]);
    }

    fn status(code: i32) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }

    /// A report with one passing, checked root.
    fn passing_report() -> Report {
        Report {
            roots: vec![RootVerdict {
                root: "a".into(),
                instance: "a".into(),
                verdict: Verdict::Pass,
                environment: Some(Environment {
                    panic_strategy: PanicStrategy::Unwind,
                    opt_level: "No".into(),
                    mir_opt_level: 1,
                    overflow_checks: true,
                    debug_assertions: true,
                    target_triple: "x86_64-unknown-linux-gnu".into(),
                    rustc_version: "1.99.0-nightly".into(),
                    all_crates: false,
                    build_std: false,
                }),
            }],
            panic_strategy: Some(PanicStrategy::Unwind),
            ..Report::default()
        }
    }

    /// Runs `finalize` and returns its result with the report it persisted.
    fn finalize_in_temp(
        label: &str,
        report: Report,
        cargo_exit: i32,
        mixed_sysroot: bool,
        warn_only: bool,
    ) -> Result<(Result<()>, Report)> {
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_finalize_{}_{label}",
            std::process::id()
        ));
        let path = directory.join("target/no-alloc/report.json");
        let result = finalize(report, &path, status(cargo_exit), mixed_sysroot, warn_only);
        let persisted = serde_json::from_reader(File::open(&path)?)?;
        fs::remove_dir_all(directory)?;
        Ok((result, persisted))
    }

    #[test]
    fn finalize_passes_a_clean_run_and_persists_it() -> Result<()> {
        let (result, persisted) = finalize_in_temp("clean", passing_report(), 0, false, false)?;
        result?;
        assert!(persisted.is_success());
        assert!(persisted.warnings.is_empty());
        Ok(())
    }

    #[test]
    fn finalize_persists_a_failed_build_before_failing_on_it() -> Result<()> {
        let (result, persisted) =
            finalize_in_temp("cargo_failed", passing_report(), 101, false, true)?;
        // Even `--warn-only` cannot excuse a build that did not finish.
        let error = result.err().context("a failed build must fail the run")?;
        assert!(error.to_string().contains("Cargo exited"), "{error}");
        assert!(!persisted.is_success());
        Ok(())
    }

    #[test]
    fn finalize_taints_in_order_so_a_mixed_sysroot_is_not_double_counted() -> Result<()> {
        // Both the build and the sysroot are bad: the first taint already
        // makes the report a failure, so the second must stay silent.
        let (result, persisted) = finalize_in_temp("mixed", passing_report(), 101, true, true)?;
        let error = result.err().context("a mixed sysroot must fail the run")?;
        assert!(
            error.to_string().contains("precompiled standard"),
            "{error}"
        );
        assert_eq!(persisted.selection_errors.len(), 1, "{persisted:?}");
        assert!(persisted.selection_errors[0].contains("immediate-abort"));
        Ok(())
    }

    #[test]
    fn finalize_records_a_zero_root_run_as_a_warning_not_a_failure() -> Result<()> {
        let (result, persisted) =
            finalize_in_temp("zero_roots", Report::default(), 0, false, false)?;
        result?;
        assert!(persisted.is_success());
        assert_eq!(persisted.warnings, [ZERO_ROOTS_WARNING]);
        Ok(())
    }

    #[test]
    fn finalize_downgrades_findings_only_under_warn_only() -> Result<()> {
        let failing = || Report {
            selection_errors: vec!["finding".into()],
            ..passing_report()
        };
        let (strict, persisted) = finalize_in_temp("strict", failing(), 0, false, false)?;
        assert!(strict.is_err());
        assert!(!persisted.is_success());
        let (lenient, persisted) = finalize_in_temp("lenient", failing(), 0, false, true)?;
        lenient?;
        // The file still says what happened; only the exit status relents.
        assert!(!persisted.is_success());
        Ok(())
    }

    #[test]
    fn refuse_configuration_is_a_no_op_without_reasons_and_records_them_otherwise() -> Result<()> {
        let path = std::env::temp_dir()
            .join(format!("cargo_no_alloc_refuse_cfg_{}", std::process::id()))
            .join("target/no-alloc/report.json");
        refuse_configuration(&path, Vec::new())?;
        assert!(!path.exists());
        let error = refuse_configuration(&path, vec!["b".into(), "a".into()])
            .err()
            .context("reasons must refuse the run")?;
        assert!(error.to_string().contains("nothing was built"), "{error}");
        let persisted: Report = serde_json::from_reader(File::open(&path)?)?;
        assert_eq!(persisted.selection_errors, ["a", "b"]);
        fs::remove_dir_all(
            path.ancestors()
                .nth(3)
                .context("report path has a root directory")?,
        )?;
        Ok(())
    }
}
