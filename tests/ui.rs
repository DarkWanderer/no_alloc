//! Drives the built `cargo-no-alloc` binary against every fixture crate in
//! `tests/ui/<case>/`. Two assertions per case:
//!
//! - `expected.json`: the primary assertion. A normalized projection of
//!   `report.json` (spans stripped — a rendered span embeds an absolute
//!   toolchain path, which isn't portable across machines; the chain of
//!   `def_path`s plus the verdict kind/reason is the stable, meaningful
//!   part). This is what should catch a real regression.
//! - `expected.stderr`: a snapshot of the full rendered diagnostic output,
//!   including real spans. rustc renders stdlib notes with an absolute
//!   sysroot path, which varies by machine (`/home/you/.rustup/...` locally
//!   vs. `/home/runner/.rustup/...` on CI); [`diagnostic_only`] replaces the
//!   active `rustc --print sysroot` with a `<sysroot>` placeholder so the
//!   snapshot is portable. Re-bless with `NO_ALLOC_BLESS=1` when the
//!   toolchain changes and the rendered diagnostics genuinely differ.
//!
//! Requires `cargo build --workspace` to have already produced
//! `cargo-no-alloc`/`no-alloc-driver` in `target/<profile>/` —
//! `cargo test --workspace` does this automatically. Located via
//! `CARGO_MANIFEST_DIR` (this package *is* the workspace root) rather than
//! a `current_exe()` sibling-walk: some sandboxes place test binaries under
//! `target/<profile>/build/<pkg>/<hash>/out/`, not the usual
//! `target/<profile>/deps/`, which breaks a fixed "walk up two directories"
//! assumption.

use no_alloc_report::{Report, Verdict};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

static CHECKER_TEST_LOCK: Mutex<()> = Mutex::new(());

/// The active toolchain's sysroot (e.g.
/// `/home/you/.rustup/toolchains/nightly-2026-08-01-x86_64-unknown-linux-gnu`),
/// used to scrub machine-specific rustlib source paths out of diagnostic
/// snapshots. `rust-toolchain.toml` at the workspace root pins the channel,
/// so this resolves to the same toolchain `cargo-no-alloc` itself invokes.
fn sysroot() -> &'static str {
    static SYSROOT: OnceLock<String> = OnceLock::new();
    SYSROOT.get_or_init(|| {
        let output = Command::new("rustc")
            .arg("--print")
            .arg("sysroot")
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("run rustc --print sysroot");
        assert!(output.status.success(), "rustc --print sysroot failed");
        String::from_utf8(output.stdout)
            .expect("rustc sysroot output is UTF-8")
            .trim()
            .to_owned()
    })
}

/// Resolved relative to this test binary's own location first, because that
/// is the only thing that tracks whichever target directory actually built
/// the current run. `cargo llvm-cov` builds into `target/llvm-cov-target/`
/// but does not export a matching `CARGO_TARGET_DIR` to the test process, so
/// resolving from the env var alone silently picks up the stale,
/// uninstrumented copy an earlier `cargo build --workspace` left in plain
/// `target/debug/` — the fixtures then pass while exercising the wrong
/// binary and contributing no coverage at all.
///
/// Every ancestor is searched rather than a fixed number of levels, since
/// some sandboxes place test binaries under
/// `target/<profile>/build/<pkg>/<hash>/out/` instead of `target/<profile>/deps/`.
fn cargo_no_alloc_bin() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors() {
            let candidate = ancestor.join("cargo-no-alloc");
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    // `PROFILE` is only available to build scripts, not regular crates, so
    // `debug_assertions` is the standard stand-in for "debug vs release".
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    target_dir().join(profile).join("cargo-no-alloc")
}

fn target_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(dir);
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target")
}

fn ui_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ui")
}

/// Keeps only the rustc-emitted diagnostic text. Raw stderr also carries
/// cargo's own build-progress lines (`Compiling`/`Updating crates.io
/// index`/`Locking N packages`/`Finished`), which vary with local registry
/// cache state and would make the snapshot flaky for reasons that have
/// nothing to do with this tool's own output.
fn diagnostic_only(stderr: &str) -> String {
    const CARGO_NOISE_PREFIXES: &[&str] = &[
        "Compiling",
        "Updating",
        "Locking",
        "Finished",
        "Adding",
        "Downloading",
        "Downloaded",
        "Fresh",
        "Blocking",
        "Removed",
        "no_alloc: checked",
        "Error: cargo exited with exit status",
        "Error: Cargo exited with exit status",
        "Error: no_alloc check failed",
    ];
    let filtered = stderr
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !CARGO_NOISE_PREFIXES
                .iter()
                .any(|prefix| trimmed.starts_with(prefix))
        })
        .collect::<Vec<_>>()
        .join("\n");
    filtered.replace(sysroot(), "<sysroot>")
}

/// Strips spans (see module doc) and keeps `expected.json` deterministic
/// regardless of iteration order.
fn normalize(mut report: Report) -> Report {
    for root in &mut report.roots {
        match &mut root.verdict {
            Verdict::Violation { chain } | Verdict::Rejected { chain, .. } => {
                for frame in chain {
                    frame.span = None;
                }
            }
            Verdict::Pass | Verdict::NotInstantiated => {}
        }
    }
    report
        .roots
        .sort_by(|a, b| (&a.root, &a.instance).cmp(&(&b.root, &b.instance)));
    report
}

fn run_case(bin: &Path, case_dir: &Path, bless: bool) -> Result<(), String> {
    let target_dir = case_dir.join("target/no-alloc");
    let _ = fs::remove_dir_all(&target_dir);

    let output = Command::new(bin)
        .arg("build")
        .current_dir(case_dir)
        .env_remove("NO_ALLOC_WARN_ONLY")
        // cargo-no-alloc refuses to run when a rustc wrapper is already set,
        // because it needs that slot for its own driver. `cargo llvm-cov`
        // sets `RUSTC_WRAPPER` for instrumentation, which would otherwise
        // make every fixture bail before compiling anything. `checker()`
        // below strips these for the same reason.
        .env_remove("RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .env("NO_ALLOC_LOG", "off")
        // The expected.stderr snapshot is plain text; forcing color off
        // here keeps the comparison stable regardless of the caller's own
        // `CARGO_TERM_COLOR` (CI sets `always`, which would otherwise leak
        // ANSI escapes into the diagnostic and break every snapshot).
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .map_err(|e| format!("failed to spawn cargo-no-alloc: {e}"))?;
    let stderr = diagnostic_only(&String::from_utf8_lossy(&output.stderr));

    let report_path = target_dir.join("report.json");
    // A bless run must never manufacture expectations from a checker that
    // never ran. Without this guard a checker that aborts before producing a
    // report blesses every fixture to an empty verdict set plus the abort
    // message — which has happened: a `RUSTC_WRAPPER` collision made all 20
    // cases bail at once and the bless silently destroyed the whole suite.
    if bless && !report_path.is_file() {
        return Err(format!(
            "refusing to bless: the checker produced no {} (exit status {:?}); \
             fix the invocation before blessing.\n--- stderr ---\n{stderr}",
            report_path.display(),
            output.status.code(),
        ));
    }

    let expected_stderr_path = case_dir.join("expected.stderr");
    if bless {
        fs::write(&expected_stderr_path, &stderr)
            .map_err(|e| format!("writing expected.stderr: {e}"))?;
    } else if expected_stderr_path.is_file() {
        let expected = fs::read_to_string(&expected_stderr_path)
            .map_err(|e| format!("reading expected.stderr: {e}"))?;
        if expected.trim_end() != stderr.trim_end() {
            return Err(format!(
                "stderr mismatch (NO_ALLOC_BLESS=1 to update)\n--- expected ---\n{expected}\n--- actual ---\n{stderr}"
            ));
        }
    }

    let report = if report_path.is_file() {
        let raw =
            fs::read_to_string(&report_path).map_err(|e| format!("reading report.json: {e}"))?;
        serde_json::from_str(&raw).map_err(|e| format!("parsing report.json: {e}"))?
    } else {
        Report::default()
    };
    let should_succeed = report.is_success();
    let normalized = normalize(report);

    let expected_json_path = case_dir.join("expected.json");
    if bless {
        let mut json = serde_json::to_string_pretty(&normalized).map_err(|e| e.to_string())?;
        json.push('\n');
        fs::write(&expected_json_path, json).map_err(|e| format!("writing expected.json: {e}"))?;
    } else {
        let raw = fs::read_to_string(&expected_json_path).map_err(|e| {
            format!("missing expected.json ({e}); run with NO_ALLOC_BLESS=1 to create it")
        })?;
        let expected: Report =
            serde_json::from_str(&raw).map_err(|e| format!("parsing expected.json: {e}"))?;
        if expected != normalized {
            return Err(format!(
                "report.json mismatch (NO_ALLOC_BLESS=1 to update)\n--- expected ---\n{expected:#?}\n--- actual ---\n{normalized:#?}"
            ));
        }
    }

    if output.status.success() != should_succeed {
        return Err(format!(
            "cargo exit status ({:?}) doesn't match report.json verdicts (expected success={should_succeed})",
            output.status
        ));
    }

    Ok(())
}

#[test]
fn ui_matrix() {
    let _guard = CHECKER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let bin = cargo_no_alloc_bin();
    assert!(
        bin.is_file(),
        "cargo-no-alloc not found at {}; run `cargo build --workspace` first",
        bin.display()
    );

    let bless = std::env::var_os("NO_ALLOC_BLESS").is_some();
    let mut cases: Vec<PathBuf> = fs::read_dir(ui_dir())
        .expect("read tests/ui")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.is_dir())
        .collect();
    cases.sort();
    assert!(!cases.is_empty(), "no fixtures found under tests/ui");

    let mut failures = Vec::new();
    for case_dir in cases {
        let name = case_dir.file_name().unwrap().to_string_lossy().into_owned();
        if let Err(msg) = run_case(&bin, &case_dir, bless) {
            failures.push(format!("[{name}] {msg}"));
        }
    }

    assert!(failures.is_empty(), "\n\n{}\n", failures.join("\n\n"));
}

/// The zero-footprint claim (ADR 0002): `#[no_alloc_check::no_alloc]` on stable,
/// with no checker involved at all, must compile clean — no
/// `feature(register_tool)` residue, no restriction on what the function
/// actually does. Uses `direct_alloc` deliberately: a function the checker
/// *would* flag, to prove the marker imposes nothing outside the checker
/// build.
#[test]
fn stable_build_has_zero_footprint() {
    let case_dir = ui_dir().join("direct_alloc");
    let target_dir = case_dir.join("target/stable-check");
    let _ = fs::remove_dir_all(&target_dir);

    let output = Command::new("cargo")
        .arg("+stable")
        .arg("build")
        .arg("--target-dir")
        .arg(&target_dir)
        .current_dir(&case_dir)
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            eprintln!(
                "skipping stable_build_has_zero_footprint: failed to spawn `cargo +stable`: {e}"
            );
            return;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("toolchain 'stable") {
            eprintln!("skipping stable_build_has_zero_footprint: no stable toolchain installed");
            return;
        }
        panic!(
            "cargo +stable build failed for an annotated crate; the marker should be zero-footprint on stable:\n{stderr}"
        );
    }

    let _ = fs::remove_dir_all(&target_dir);
}

fn checker(case: &str, args: &[&str]) -> std::process::Output {
    Command::new(cargo_no_alloc_bin())
        .args(args)
        .current_dir(ui_dir().join(case))
        .env("NO_ALLOC_LOG", "off")
        .env_remove("NO_ALLOC_WARN_ONLY")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .output()
        .expect("run cargo-no-alloc")
}

#[test]
fn cached_warn_only_does_not_hide_strict_failure() {
    let _guard = CHECKER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let warning = checker("direct_alloc", &["--warn-only", "--", "build"]);
    assert!(
        warning.status.success(),
        "{}",
        String::from_utf8_lossy(&warning.stderr)
    );
    let report_path = ui_dir().join("direct_alloc/target/no-alloc/report.json");
    let warning_report = fs::read(&report_path).unwrap();
    let strict = checker("direct_alloc", &["--", "build"]);
    assert!(!strict.status.success());
    assert_eq!(warning_report, fs::read(&report_path).unwrap());
    let report: Report = serde_json::from_reader(fs::File::open(report_path).unwrap()).unwrap();
    assert!(!report.is_success());
}

#[test]
fn unmatched_and_non_function_roots_are_reported() {
    let _guard = CHECKER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let selected = checker(
        "pure_arith",
        &["--root", "pure_arith::selected", "--", "build"],
    );
    assert!(selected.status.success());
    let selected_report: Report = serde_json::from_reader(
        fs::File::open(ui_dir().join("pure_arith/target/no-alloc/report.json")).unwrap(),
    )
    .unwrap();
    assert!(selected_report
        .roots
        .iter()
        .any(|root| root.root == "pure_arith::selected"));

    let unmatched = checker("pure_arith", &["--root", "missing::root", "--", "build"]);
    assert!(!unmatched.status.success());
    assert!(String::from_utf8_lossy(&unmatched.stderr).contains("did not match any item"));

    let non_function = checker(
        "pure_arith",
        &["--root", "pure_arith::VALUE", "--", "build"],
    );
    assert!(!non_function.status.success());
    assert!(String::from_utf8_lossy(&non_function.stderr).contains("is not a function"));

    let warning = checker(
        "pure_arith",
        &["--warn-only", "--root", "missing::root", "--", "build"],
    );
    assert!(warning.status.success());
}

#[test]
fn rejects_check_and_forwards_test_runner_arguments() {
    let _guard = CHECKER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert!(!checker("pure_arith", &["--", "check"]).status.success());
    let test = checker("pure_arith", &["--", "test", "--", "--list"]);
    assert!(
        test.status.success(),
        "{}",
        String::from_utf8_lossy(&test.stderr)
    );
}

#[test]
fn cargo_external_subcommand_argv_is_supported() {
    let _guard = CHECKER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = std::env::var_os("PATH").unwrap_or_default();
    let joined = std::env::join_paths(
        std::iter::once(target_dir().join(if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }))
        .chain(std::env::split_paths(&path)),
    )
    .unwrap();
    let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["no-alloc", "--", "build"])
        .current_dir(ui_dir().join("pure_arith"))
        .env("PATH", joined)
        .env("NO_ALLOC_LOG", "off")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .output()
        .expect("run cargo no-alloc");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn environment_interfaces_encoded_flags_and_all_crates_work() {
    let _guard = CHECKER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let environment = Command::new(cargo_no_alloc_bin())
        .args(["--", "build"])
        .current_dir(ui_dir().join("direct_alloc"))
        .env("NO_ALLOC_LOG", "off")
        .env("NO_ALLOC_WARN_ONLY", "1")
        .env("NO_ALLOC_ROOTS", "direct_alloc::root")
        .env("CARGO_ENCODED_RUSTFLAGS", "-C\x1fdebuginfo=0")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .output()
        .expect("run cargo-no-alloc with environment interfaces");
    assert!(
        environment.status.success(),
        "{}",
        String::from_utf8_lossy(&environment.stderr)
    );

    let all_crates = checker("pure_arith", &["--all-crates", "--", "build"]);
    assert!(
        all_crates.status.success(),
        "{}",
        String::from_utf8_lossy(&all_crates.stderr)
    );
}

#[test]
fn multi_crate_report_is_deterministic() {
    let _guard = CHECKER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let first = checker("multi_crate", &["--", "build"]);
    assert!(first.status.success());
    let report_path = ui_dir().join("multi_crate/target/no-alloc/report.json");
    let first_report = fs::read(&report_path).unwrap();

    let second = checker("multi_crate", &["--", "build"]);
    assert!(second.status.success());
    assert_eq!(first_report, fs::read(report_path).unwrap());
}
