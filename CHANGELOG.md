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


### Fixed

- The UI fixture harness no longer blesses expectations from a checker run
  that produced no report. Previously a checker that aborted before writing
  `report.json` would, under `NO_ALLOC_BLESS=1`, overwrite every fixture with
  an empty verdict set and the abort message.
- `tests/ui.rs` strips inherited `RUSTC_WRAPPER`/`RUSTC_WORKSPACE_WRAPPER`
  before invoking the checker, and resolves the built binary relative to the
  test executable rather than trusting `CARGO_TARGET_DIR`.

### Changed

- Declared MSRV for `no_alloc_check` and `no_alloc_report` raised to 1.88,
  matching what their test suites actually require.