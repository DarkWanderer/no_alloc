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
before a test-runner `--` separator.

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

Every rustc process atomically writes a uniquely named report fragment. The
wrapper merges fragments deterministically, removes duplicates and superseded
`NotInstantiated` entries, validates every requested root across the complete
build, and atomically writes `target/no-alloc/report.json`. Concurrent compiler
processes never share an output filename. Report write failures are operational
errors even under `--warn-only`.

## Traversal

Traversal is per monomorphized `Instance`, with cycle detection keyed by the
instance rather than `DefId`. Allocator leaf detection precedes MIR-availability
checks because allocator shims are declarations without ordinary MIR.

Resolved calls, tail calls, and required drop glue are traversed. Function
pointers, virtual dispatch, MIR-less foreign calls, and inline assembly reject.
All edges in a body are considered; allocation violations take priority over
rejections.

The terminator classification is documented in
[ADR 0003](../adr/0003-reject-unresolved-edges.md). In particular, terminal
unwind control flow is not a call. Assertions pass only under a compiler-known
non-unwinding panic strategy and otherwise reject. That "pass" is a scope
exclusion, not a proof: the traversal's guarantee covers a root's
non-panicking execution paths only (`README.md`, "Guarantee and
limitations").

## Verification

Unit tests cover macro shapes, CLI parsing, report merging, and stable report
logic. Full toy crates under `tests/ui/` are authoritative for rustc-private
behavior, including cross-crate roots and multi-crate aggregation. The
throughput harness performs five real checker rebuilds and reports the
median instances/second; there is no fixed reference number, since it is
machine-dependent — run `cargo bench` locally to get one.
