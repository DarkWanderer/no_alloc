# Design

## Packages

- `no_alloc_check` is the stable-compatible proc macro. Its marker is inert
  unless the checker supplies `cfg(no_alloc_check)` and registers the
  `no_alloc_tool` attribute namespace.
- `no_alloc_report` contains stable serde/report and root-spec logic.
- `no_alloc_analysis` contains rustc-private root discovery and traversal.
- `cargo-no-alloc` installs both `cargo-no-alloc` and `no-alloc-driver`.

The rustc-private boundary is kept out of the macro and report crates. The
driver and analysis require exactly `nightly-2026-08-01` and Linux.

## Invocation

Cargo external subcommands receive `no-alloc` as their first forwarded
argument; the wrapper removes it before parsing checker flags. Only `build`
and `test` are accepted because `check` does not populate the codegen mono
graph. Forced `--target`, `--target-dir`, and `-Zbuild-std` options are placed
before a test-runner `--` separator. `--immediate-abort` implies
`-Zbuild-std` and adds `-Zunstable-options -Cpanic=immediate-abort` to the
injected rustc flags, so the checked crate's own manifest needs no
nightly-only keys.

The wrapper uses the executable named by `CARGO`, preserves existing Rust
flags, and refuses to overwrite existing rustc-wrapper configuration. An
exclusive `target/no-alloc.lock` prevents concurrent checker runs in one
workspace. Normal mode cleans workspace-member artifacts in the dedicated
target; `--all-crates` cleans the complete checker target. This guarantees that
checker configuration or binary changes cannot be hidden by Cargo's cache.

## Roots and reports

The driver scans `collect_and_partition_mono_items` for local and foreign
annotated function `Instance`s. This discovers generic dependency roots at the
downstream crate where each concrete instantiation exists. Uncalled,
non-generic local roots are seeded directly; uncalled generic definitions are
reported as `NotInstantiated` unless another fragment contains an instance.

Every rustc process atomically writes a uniquely named report fragment. Each
checked root records the `Environment` (panic strategy, opt-level,
mir-opt-level, overflow-checks, debug-assertions, target triple, rustc
version, `--all-crates`/`--build-std`) its verdict was proven under;
`NotInstantiated` markers have no verdict and no environment. This per-root association preserves legitimate differences
between host and target artifacts in one Cargo build. A checked root from a
legacy fragment with no recorded environment becomes a `selection_error`
rather than being silently relabelled. The wrapper merges fragments
deterministically, removes duplicates and superseded `NotInstantiated`
entries, validates every requested root across the complete build, and
atomically writes `target/no-alloc/report.json`. Concurrent compiler processes
never share an output filename.
Report write failures are operational errors even under `--warn-only`.

## Traversal

Traversal is per monomorphized `Instance`, with cycle detection keyed by the
instance rather than `DefId`. Allocator leaf detection precedes MIR-availability
checks because allocator shims are declarations without ordinary MIR.

Resolved calls, tail calls, compiler-generated shims, and required drop glue
are traversed. A call edge resolves from the callee operand's type, so a
function item passed as a callback is followed like any other callee
([ADR 0007](../adr/0007-shim-and-fn-item-resolution.md)). Function pointers,
virtual dispatch, MIR-less foreign calls, and inline assembly reject.
Intrinsics are classified against the non-allocating intrinsic table in
`no_alloc_report` rather than rejected wholesale
([ADR 0005](../adr/0005-intrinsic-leaf-classification.md)); an intrinsic not
in the table rejects, named. All edges in a body are considered; allocation
violations take priority over rejections.

The terminator classification is documented in
[ADR 0003](../adr/0003-reject-unresolved-edges.md). In particular, terminal
unwind control flow is not a call. Assertions pass only under a compiler-known
non-unwinding panic strategy and otherwise reject. That "pass" is a scope
exclusion, not a proof, and it is drawn by terminator shape: an explicit
`panic!()` or `.unwrap()` is an ordinary `Call` terminator that remains in
scope and rejects if unresolved (`README.md`, "Guarantee and limitations").
Under `--immediate-abort`
([ADR 0006](../adr/0006-immediate-abort-checking-mode.md)), which rebuilds
std with `-Cpanic=immediate-abort` so panic paths are real, walkable edges
ending in `intrinsics::abort` and need no exclusion.
[`iterators.md`](iterators.md) is the measured consequence.

## Verification

Unit tests cover macro shapes, CLI parsing, report merging, and stable report
logic. Full toy crates under `tests/ui/` are authoritative for rustc-private
behavior, including cross-crate roots and multi-crate aggregation. The
throughput harness performs five real checker rebuilds and reports the
median instances/second; there is no fixed reference number, since it is
machine-dependent — run `cargo bench` locally to get one.
