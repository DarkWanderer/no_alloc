//! Wall-clock harness (`harness = false` — not a criterion micro-bench):
//! runs `cargo-no-alloc build` against `benches/fixtures/throughput/`, a
//! generated crate with 1365 functions in a depth-5, branching-4 call tree,
//! all pure arithmetic (deliberately: a violation would let the DFS's
//! violation-wins-early-exit short-circuit before visiting the whole
//! graph, defeating the point of a throughput measurement). Reports
//! instances/sec so a traversal regression that scales badly with mono
//! graph size shows up before someone points this at a real workspace.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const FIXTURE_FN_COUNT: u64 = 1365;
const SAMPLE_COUNT: usize = 5;

fn cargo_no_alloc_bin() -> PathBuf {
    // `CARGO_MANIFEST_DIR` here is the workspace root (this package is the
    // root package), so this is more robust than a `current_exe()`
    // sibling-walk — some sandboxes place bench/test binaries under
    // `target/<profile>/build/<pkg>/<hash>/out/`, not the usual
    // `target/<profile>/deps/`.
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join("target"));
    // Keep the measured binary profile fixed across runs. The documented
    // prerequisite builds the checker in dev mode.
    target_dir.join("debug").join("cargo-no-alloc")
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("benches/fixtures/throughput")
}

fn run(bin: &Path, dir: &Path) -> std::process::ExitStatus {
    Command::new(bin)
        .arg("build")
        .current_dir(dir)
        .env_remove("NO_ALLOC_WARN_ONLY")
        .env_remove("NO_ALLOC_LOG")
        .status()
        .expect("failed to spawn cargo-no-alloc")
}

fn main() {
    let bin = cargo_no_alloc_bin();
    assert!(
        bin.is_file(),
        "cargo-no-alloc not found at {}; run `cargo build --workspace` first",
        bin.display()
    );
    let dir = fixture_dir();

    // Warm up: build the no_alloc_check dependency once, uncounted.
    let status = run(&bin, &dir);
    assert!(status.success(), "warmup build failed");

    let src = dir.join("src/main.rs");
    let contents = std::fs::read_to_string(&src).expect("read fixture source");
    let mut samples = Vec::with_capacity(SAMPLE_COUNT);
    for sample in 1..=SAMPLE_COUNT {
        // Rewriting identical contents updates mtime and forces Cargo to invoke
        // the checker instead of returning a cached success.
        std::fs::write(&src, &contents).expect("touch fixture source");

        let start = Instant::now();
        let status = run(&bin, &dir);
        let elapsed = start.elapsed();
        assert!(status.success(), "timed build {sample} failed");
        let rate = FIXTURE_FN_COUNT as f64 / elapsed.as_secs_f64();
        println!(
            "sample {sample}: {FIXTURE_FN_COUNT} functions in {:.3}s ({rate:.0} instances/sec)",
            elapsed.as_secs_f64()
        );
        samples.push(rate);
    }

    samples.sort_by(f64::total_cmp);
    println!(
        "median throughput: {:.0} instances/sec",
        samples[SAMPLE_COUNT / 2]
    );
}
