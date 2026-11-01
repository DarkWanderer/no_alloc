# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Breaking

- Panic paths are out of scope by default: the default guarantee is now
  "no allocation on any non-panicking path" (ADR 0008). A call to a
  standard-library panic entry point in the new audited panic table
  (`no_alloc_report::is_panic_entry_point`: `panic_fmt`, `unwrap_failed`,
  `expect_failed`, `slice_index_fail`, the `panic_const_*` lang items,
  `begin_panic`, ...) ends the walk, and an `Assert` terminator is out of
  scope under every panic strategy, `unwind` included. `.unwrap()`,
  indexing, `panic!()` and iterator chains now pass by default when nothing
  else in them allocates, and `-- test` is usable. A diverging function that
  is not a panic (`process::abort`, `process::exit`, `handle_alloc_error`)
  is still walked or rejected.
- `--immediate-abort` is removed (an unknown option, not an alias). The same
  mode is now `--strict`: it implies `--build-std`, compiles everything with
  `-Cpanic=immediate-abort`, and also tells the analysis to walk panic paths
  instead of stopping at them.
- Fixtures `panic_out_of_scope`, `panic_message_allocates`,
  `panic_table_unknown_rejects`, `panic_table_user_path`,
  `strict_host_panic_reject` and `strict_host_fallback_upstream_opt`; the
  `*_immediate_abort` fixtures are renamed `strict_*`, and `iterator_reject`
  is removed (it now passes, covered by `panic_out_of_scope`).
- Report schema version 3: each root's `environment` records `panic_paths`
  (`"out_of_scope"` or `"checked"`). A schema-2 environment reads back as
  `"checked"`, the only behaviour it could have had.

### Added

- `--root` accepts a comma-separated list of paths and may be repeated; a
  malformed list such as `a,,b` is an argument error.
- A non-UTF-8 `RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS` is refused instead of
  being decoded lossily.
- Configuration refusals (rustc override, `RUSTC_WRAPPER`, ambient
  immediate-abort without `--build-std`, non-UTF-8 flags) persist a failing
  `report.json`; operational errors still write none.
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
- `--immediate-abort` (since replaced by `--strict`, see Breaking), which
  builds the crate and the standard library with
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
- Report schema version 2 records the compiler and checker environment on
  every checked root, including panic strategy, optimization settings, target,
  rustc version, and the `--all-crates`/`--build-std` modes.
- UI coverage for direct alloc/realloc leaves, tail calls, unknown intrinsics,
  ordinary cross-crate calls, uninstantiated dependency roots, non-workspace
  markers under `--all-crates`, clone/closure shims, explicit panic calls,
  dynamic drop glue, and thread-local access.
- `warnings` list in `report.json` (omitted when empty, additive, schema
  version unchanged). The zero-roots condition is recorded there as well as
  printed, so a reader of the file alone cannot take `roots: []` for a
  verified pass; the exit code is unchanged.
- Fixtures `extern_type_tail`, `weak_linkage_reject`, `async_root_reject`,
  `cross_crate_shared_generic`, `inlined_generic_root`, `config_build_target`,
  `config_build_rustc` and `config_env_no_alloc`.
- Fixtures `cross_crate_shared_generic_release`,
  `cross_crate_shared_generic_fnptr`, `async_root_upstream_reject`,
  `package_default_target` and `existing_rustc_attrs_feature`.


### Fixed

- Rustflags from Cargo config (`build.rustflags`, a non-empty `target.<host>.rustflags` list,
  and their `CARGO_*` environment spellings) are no longer silently dropped:
  when `RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS` is unset they are imported ahead
  of the checker's own flags, so a root guarded by `#[cfg(heap)]` is checked
  with `--cfg heap` as shipped. `target.'cfg(...)'.rustflags` is refused with
  a persisted `selection_errors` entry rather than guessed (ADR 0006).
- `build.rustc-wrapper` / `build.rustc-workspace-wrapper` in Cargo config (or
  `CARGO_BUILD_RUSTC_WRAPPER` / `CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER`) are
  ignored for the run with a warning; a wrapper that short-circuited a unit
  used to drop its roots from the report with exit 0. The wrapper variable the
  checker does not use is blanked for the build, which disables the matching
  config key. The `RUSTC_WRAPPER` variables are still refused.
- `cargo-no-alloc` now installs a `tracing` subscriber (`NO_ALLOC_LOG`,
  default off), used to log the imported config rustflags at info level.
- Intrinsics in the non-allocating table no longer hide their Rust fallback
  body: when codegen may call it (an intrinsic the backend does not lower
  itself, or `mem::swap` of a non-SSA type when optimizing), the fallback is
  walked like any other callee (ADR 0005).
- The unoptimized `mem::swap` shortcut (`typed_swap_nonoverlapping` swapped in
  place at opt-level 0) now applies only to callers this crate codegens
  itself. A dependency built at a higher opt-level (for example
  `[profile.dev.package."*"] opt-level = 1`, where share-generics stays on and
  the root reuses the dependency's instantiation) lowers the same call through its
  fallback body, which the checker previously skipped because it read the
  root crate's opt-level (ADR 0005). Precision cost: with share-generics at
  opt-level 0, a linked dependency that also instantiates the same
  `mem::swap::<T>` makes the instance upstream-shared, so a local root
  swapping a non-SSA type is now rejected even though the dependency lowered
  it in place (conservative, ADR 0003; pinned by
  `shared_generic_swap_conservative`).
- Under `--immediate-abort`, a failing `Assert` is walked into the panic lang
  item codegen calls, instead of being skipped on the assumption that it
  aborts (ADR 0003, ADR 0006).
- `cargo clean` now receives the build's `--release`/`--profile`/
  `--manifest-path`/`--config`, so a rerun of a non-dev-profile build no
  longer finds its workspace units fresh and silently drops their roots.
- A Cargo failure now marks `report.json` as failing (when nothing else
  already does), and a previous run's `report.json` is removed before any
  check that can abort the run.
- The driver fails the build for a finding with no chain to anchor a span on,
  instead of passing it silently.
- Drop glue for `dyn Trait` is rejected instead of being mistaken for a safe
  recursive cycle; the concrete destructor is selected through a vtable and
  is not statically known.
- Compiler-generated instances are classified exhaustively by `InstanceKind`
  and `ShimKind`; audited synthesized bodies are traversed, while unaudited or
  runtime-selected bodies reject by default.
- `Rvalue::ThreadLocalRef` is no longer invisible to the terminator-only call
  graph walk. It rejects because TLS access can invoke an unmodeled runtime
  operation on some targets.
- Hand-written root markers on non-functions are reported as selection errors
  instead of being treated as callable MIR, and deep call-graph traversal uses
  rustc's stack-growth guard.
- Panic-scope documentation now distinguishes excluded MIR terminators from
  explicit panic calls, which remain ordinary traversed call edges.

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
- Documented (ADR 0006) why a workspace build script or proc-macro reporting
  `unwind` while everything else reports `immediate_abort` in the same
  build is not a soundness gap: Cargo always compiles host artifacts under
  `unwind` regardless of `--immediate-abort`, and `unwind` is the *stricter*
  of the two as far as the traversal is concerned (`Assert` rejects
  outright rather than taking the `abort`/`immediate-abort` carve-out). A
  rejecting or violating host root fails that unit's own compilation
  outright — via the same hard-error mechanism ordinary findings use — so
  it can never coexist with a successful build; only a genuinely passing
  one can, and that verdict is never weaker than the target's. Pinned by
  `tests/ui/strict_host_root`.
- README and ADR 0003 no longer point at `-Zbuild-std-features=panic_immediate_abort`,
  which is a `compile_error!` on the pinned nightly — it is a panic strategy
  now, and `--immediate-abort` supplies it.

- A field projection into a type whose struct tail is an extern type is
  rejected. Codegen sizes the tail with a `panic_nounwind` call that no MIR
  edge names (ADR 0003).
- A callee with explicit `linkage`/`import_linkage` (for example
  `#[linkage = "weak"]`) is rejected: the linker may bind the symbol to a
  different definition than the MIR checked (ADR 0003).
- `async fn` roots, marked or `--root`-selected, are a selection error; only
  the future constructor has MIR, so the body that runs was never checked
  (ADR 0003).
- Marked generic roots instantiated in an upstream crate and linked to under
  share-generics are now discovered in the default (non-`--all-crates`) mode,
  where they were silently absent from the report. Non-generic markers in
  non-workspace dependencies still need `--all-crates` (ADR 0004).
- `#[no_alloc]` now also emits `rustc_no_mir_inline` under
  `cfg(no_alloc_check)` (the driver injects `feature(rustc_attrs)` and
  `allow(internal_features)`), so a marked generic root can no longer be
  MIR-inlined away and reported `NotInstantiated`, which counts as success.
  Normal builds are unaffected. A `--root`-selected unmarked generic fn is
  still exposed (ADR 0002).
- `cargo no-alloc` refuses a configured `build.target` (`[build] target`,
  `CARGO_BUILD_TARGET`, `--config build.target`), which it silently dropped
  while the shipped build compiled different `cfg(target_...)` code.
- `cargo no-alloc` refuses `RUSTC`/`CARGO_BUILD_RUSTC` and a configured
  `build.rustc` (see below), and the driver fails unless its argv[1] is the
  validated `rustc` (`NO_ALLOC_EXPECTED_RUSTC`); a substitute compiler shaped
  the shipped build but not the checked one (ADR 0006).
- `cargo no-alloc` refuses Cargo `[env]` keys starting `NO_ALLOC_` and always
  sets `NO_ALLOC_WARN_ONLY` (`1` or `0`) instead of removing it, so config
  cannot steer the driver (ADR 0006).
- A configured `build.rustc` is refused (persisted `selection_errors` entry,
  nothing built) instead of warned about and ignored: a substitute compiler
  adding `--cfg heap` shaped `cargo build` but not the checked build, so the
  report could pass code the shipped build allocates in (ADR 0006).
- Upstream-shared generic roots are found through the collector's own
  per-instance neighbour query rather than a Call/Drop-only walk, so a marked
  root reached through an upstream body at `opt-level = 3` (an
  `#[inline(never)]` wrapper), through a reify cast to a fn pointer, an unsize
  cast or a fn item in a const is no longer missed (ADR 0004).
- A `--root` naming an upstream `async fn` is reported as a selection error
  instead of being dropped after its spec was marked matched.
- `cargo no-alloc` refuses a workspace member that sets `default-target` or
  `forced-target` (`cargo-features = ["per-package-target"]`): the checker's
  `--target <host>` replaced it, so the shipped build targeted another
  platform than the one checked.
- A crate that already has `#![feature(rustc_attrs)]` is checked again: the
  injected `-Zcrate-attr=feature(rustc_attrs)` made it fail the
  deny-by-default `duplicate_features` lint, so nothing was checked. The
  checker now also injects `-Zcrate-attr=allow(duplicate_features)`.
- The zero-roots warning also fires when every root is `NotInstantiated`.
- README and ADR 0003/0006: `Unreachable` is no longer described as lowering
  to a `panic_fmt` call (it codegens to a bare LLVM `unreachable`); the
  garbled duplicate sentence in the panic-scope paragraph is fixed; the
  `Shim(DropGlue)` row now says an explicit `drop_in_place::<dyn T>` does
  construct the glue and is rejected on its residual `Drop`; ADR 0006 no
  longer says `Assert` has the `Edge::None` carve-out under `immediate-abort`.
- README/ADR 0004 no longer imply cross-crate generic roots were always
  found at downstream sites; the module comment in `traversal.rs` names the
  real memoization key, `(Instance, bool)`.
- README documents the limitations the audit left open: allocator-API calls
  other than the `__rust_alloc` entry points, checked build differing from the
  shipped build, `--root` on unmarked generics, and the linkage rule.

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
