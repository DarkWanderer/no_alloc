# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial Linux release of the monomorphized no-allocation checker.
- Stable-compatible `no_alloc_check::no_alloc` marker macro.
- Conservative call/drop analysis with per-instance diagnostics and reports.
- Cross-crate generic-root discovery and deterministic multi-crate aggregation.
- `cargo no-alloc` build/test orchestration with strict and warn-only modes.
- Warning when a run checks zero root instances, naming the two likely causes
  (no `#[no_alloc]` marker reached the build, or the marker lives outside the
  workspace and needs `--all-crates`). A green run that analyzed nothing was
  previously indistinguishable from a green run that verified something.
- Run-time verification that the project being checked is on the pinned
  toolchain, replacing a raw `librustc_driver-*.so` dynamic-linker failure
  with the same actionable message `build.rs` emits at build time.
- `--help`/`-h` and `--version`/`-V` on `cargo no-alloc`.
- `instance` field on report `Frame`, carrying the monomorphized rendering
  alongside the existing definition-level `def_path` — so `report.json`
  records which instantiation a chain frame refers to, matching what the
  rendered diagnostic already showed.
- `--immediate-abort`, which builds the crate and the standard library with
  `-Cpanic=immediate-abort` (implying `--build-std`). Panic paths then lower
  to a bare `abort()` and are traversed as ordinary edges instead of being
  excluded from the guarantee. This is what makes iterator code checkable:
  measured over 35 iterator patterns, `panic = "abort"` passes none and
  `--immediate-abort` passes 33 (ADR 0006, `docs/iterators.md`).
- Non-allocating intrinsic table (`no_alloc_report::intrinsic_cannot_reach_allocator`).
  An intrinsic is a resolved callee whose body lives in the compiler, so it is
  now classified rather than rejected for having no MIR; an intrinsic outside
  the table still rejects, and the diagnostic names it (ADR 0005).
- `docs/iterators.md` and `examples/iterators`: the measured non-allocating
  iterator subset, as prose and as 35 runnable roots.
- `panic_strategy` in `report.json`, recorded from the compilation rather
  than from the flag the user typed. A `Pass` means something different
  under each strategy, so a persisted report that omits it cannot be read
  back correctly (ADR 0006).
- UI fixtures may carry a `checker-args` file to run under a non-default
  checker configuration.


### Fixed

- The UI fixture harness no longer blesses expectations from a checker run
  that produced no report. Previously a checker that aborted before writing
  `report.json` would, under `NO_ALLOC_BLESS=1`, overwrite every fixture with
  an empty verdict set and the abort message.
- `tests/ui.rs` strips inherited `RUSTC_WRAPPER`/`RUSTC_WORKSPACE_WRAPPER`
  before invoking the checker, and resolves the built binary relative to the
  test executable rather than trusting `CARGO_TARGET_DIR`.
- `tests/ui.rs` also strips `RUST_BACKTRACE` and `RUST_LIB_BACKTRACE`, which
  otherwise appended anyhow's machine-specific backtrace to every fixture's
  stderr snapshot and failed the whole matrix on developer machines that
  export either one.
- The validity assertions (`assert_inhabited`, `assert_zero_valid`,
  `assert_mem_uninitialized_valid`) are no longer treated as unconditional
  non-allocating leaves. Codegen emits a `panic_nounwind` call for an
  instantiation that fails the requirement, which the traversal never sees,
  so they are classified per instantiation with the same query codegen uses.
- The panic strategy in `report.json` is claimed only by fragments that
  carry verdicts. Cargo does not pass the target `RUSTFLAGS` to host units,
  so a wrapped build script or proc macro compiles under a different
  strategy; its empty fragment used to disagree with the real ones and drop
  the field from the merged report entirely.
- `--immediate-abort -- test` is rejected during argument parsing instead of
  failing inside rustc with `building tests with panic=abort is not
  supported without -Zpanic_abort_tests`, minutes into a sysroot rebuild.
- README and ADR 0003 no longer point at `-Zbuild-std-features=panic_immediate_abort`,
  which is a `compile_error!` on the pinned nightly — it is a panic strategy
  now, and `--immediate-abort` supplies it.

### Changed

- Body availability is decided by `InstanceKind` rather than by asking
  `is_mir_available` about the instance's `DefId`, and a call edge resolves
  from the callee operand's type rather than only from a MIR constant. Shim
  instances and callbacks passed as function items are followed instead of
  rejected — `Iterator::max`, `min_by_key`, `flat_map` and `.scan(..).last()`
  become checkable, with function pointers and virtual dispatch still
  rejecting (ADR 0007).
- Declared MSRV for `no_alloc_check` and `no_alloc_report` raised to 1.88,
  matching what their test suites actually require.