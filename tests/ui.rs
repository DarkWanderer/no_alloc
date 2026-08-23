//! Drives the built `cargo-no-alloc` binary against every fixture crate in
//! `tests/ui/<case>/`. Two assertions per case:
//!
//! - `expected.json`: the primary assertion. A normalized projection of
//!   `report.json` (spans stripped — a rendered span embeds an absolute
//!   toolchain path, which isn't portable across machines; the chain of
//!   `def_path`s plus the verdict kind/reason is the stable, meaningful
//!   part). This is what should catch a real regression.
//! - `expected.stderr`: a snapshot of the full rendered diagnostic output,
//!   including real spans. Machine-specific by nature (rustup install
//!   paths), so it's explicitly a snapshot, not a portability guarantee —
//!   re-bless with `NO_ALLOC_BLESS=1` when the toolchain or its install
//!   path changes.
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

fn cargo_no_alloc_bin() -> PathBuf {
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
        "Error: cargo exited with exit status",
    ];
    stderr
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !CARGO_NOISE_PREFIXES
                .iter()
                .any(|prefix| trimmed.starts_with(prefix))
        })
        .collect::<Vec<_>>()
        .join("\n")
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
        .env("NO_ALLOC_LOG", "off")
        .output()
        .map_err(|e| format!("failed to spawn cargo-no-alloc: {e}"))?;
    let stderr = diagnostic_only(&String::from_utf8_lossy(&output.stderr));

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

    let report_path = target_dir.join("report.json");
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

/// The zero-footprint claim (ADR 0002): `#[no_alloc::no_alloc]` on stable,
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
