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
- The `cargo_no_alloc` tests that write a fake `rustc` and then execute it
  are serialized. Run in parallel they raced: one test's `Command::spawn`
  inherits the sibling's still-open write fd, and the exec fails with
  `ETXTBSY`, so whichever test lost reported a spawn error instead of the
  verdict it asserts on. Reproduced at roughly one run in two.
- `immediate_abort_rustflags_are_added_only_when_asked_for` now clears and
  restores `RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS` under the same lock as the
  test above, instead of reading whatever the process environment happened
  to hold. It was reading ambient state unguarded, so a developer's own
  shell (or the sibling test's transient mutation) could leak in and fail
  its "unasked-for" assertions spuriously.
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
  checked at least one instance. Cargo does not pass the target `RUSTFLAGS`
  to host units, so a wrapped build script or proc macro compiles under a
  different strategy; its fragment used to disagree with the real ones and
  drop the field from the merged report entirely — including when all it
  held was a `NotInstantiated` marker. Merging keeps the same definition: a
  report with real verdicts but no strategy leaves the result unknown rather
  than borrowing a sibling's, so `merge` is associative, while fragments
  holding only markers stay neutral.
- `cargo no-alloc` rejects an environment that already sets
  `-Cpanic=immediate-abort` without `--immediate-abort`, in any spelling
  rustc accepts (`-Cpanic=…`, `-C panic=…`, `--codegen …`, in either flag
  variable). Nothing rebuilds
  the sysroot in that configuration, so the crate compiled under
  immediate-abort while std kept its precompiled panic runtime, and the
  report claimed `immediate_abort` for a build where std's panic paths were
  not compiled that way.
- `--immediate-abort` combined with a test-harness target — `-- test`, or
  `-- build` with `--tests`/`--test`/`--benches`/`--bench`/`--all-targets` —
  is rejected during argument parsing instead of failing inside rustc with
  `building tests with panic=abort is not supported without
  -Zpanic_abort_tests`, minutes into a sysroot rebuild.
- The float entries in the intrinsic table match the exact spelling each
  name has in the intrinsic set. Normalizing away trailing underscores meant
  a future `sqrt_f32` or `round_ties_evenf64` would have been accepted
  without anyone auditing its lowering, which is not what an allowlist is
  for. All 99 float names in the pinned toolchain still match.
- The checker's own `-Cpanic=immediate-abort` flag is now appended
  unconditionally rather than skipped when an equal flag already appears
  somewhere earlier. `-Cpanic` is last-wins, so an inherited
  `-Cpanic=immediate-abort ... -Cpanic=unwind` used to satisfy the
  "already present" test while the build still ran under unwind, silently
  not applying `--immediate-abort` at all.
- `--immediate-abort`'s test-harness-target guard no longer rejects
  `--test`/`--bench`/`--benches`: those name a target that may set
  `harness = false` (as this repository's own benchmarks do) and build
  fine under an abort strategy. Only `--tests` and `--all-targets`, which
  always pull in a libtest harness, are still rejected pre-flight; a
  narrower selection that does need a harness is caught by the
  observed-strategy check below instead.
- `cargo no-alloc` also rejects a build whose driver reported compiling
  under `-Cpanic=immediate-abort` without `--build-std` — the case the
  ambient-flag guard cannot see because the strategy came from the
  manifest (`cargo-features = ["panic-immediate-abort"]` plus a profile),
  `config.toml`, or a narrow `--test`/`--bench` selection.
  `--build-std` on its own is accepted: it rebuilds the sysroot under the
  same flags, so a hand-set strategy is coherent. This check reads every
  fragment directly rather than the merged report's `panic_strategy`: a
  target-specific config can select the strategy for the checked crate
  while a wrapped host unit compiles under something else, and
  `Report::merge` reports that disagreement as unknown for `report.json`'s
  sake — which would otherwise let the same mix through this guard.
- `declares_immediate_abort` (the ambient-flag guard) now resolves the
  *last* `-C panic=...` setting in the flag stream, matching rustc's
  last-wins behavior, instead of flagging the environment as soon as
  `immediate-abort` appears anywhere. An environment ending in
  `-Cpanic=unwind` after an earlier `-Cpanic=immediate-abort` compiles
  under `unwind` and was being rejected for a strategy that was not
  actually in effect.
- The same ambient-flag guard's escape hatch is `--build-std`, not
  `--immediate-abort` specifically. `--immediate-abort` always implies
  `--build-std`, but the reverse isn't required, and plain `--build-std`
  alongside a hand-set ambient strategy rebuilds the sysroot under the same
  inherited flags — exactly as coherent as the flag, and already treated
  that way by the post-build check. Gating on `--immediate-abort` instead
  rejected that legitimate combination before the build ever ran.
- A mixed-sysroot rejection is now recorded in the persisted `report.json`
  (via `selection_errors`), not only raised as a process error. Previously
  `report.json` was written before the check ran, so a trivial root that
  passes under any panic strategy would be written as a bare `Pass` with an
  `ImmediateAbort` label — `Report::is_success()` on the file alone said
  `true` even though the process producing it exited nonzero. A tool
  reading the JSON directly, rather than this process's exit code, would
  have seen a false success.
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