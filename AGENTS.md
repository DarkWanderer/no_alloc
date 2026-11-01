# no_alloc — notes for coding agents

Rules that bind every contributor live in [CONTRIBUTING.md](CONTRIBUTING.md);
follow them. In particular:

- Soundness ("reject, don't assume"; per-instantiation verdicts; the
  intrinsic and panic tables are allowlists — never add a name to make
  something pass): [CONTRIBUTING.md#soundness](CONTRIBUTING.md#soundness),
  ADRs [0001](adr/0001-mono-site-analysis.md),
  [0003](adr/0003-reject-unresolved-edges.md),
  [0005](adr/0005-intrinsic-leaf-classification.md),
  [0008](adr/0008-panic-paths-out-of-scope-by-default.md).
- Pinned nightly and the stable-only crates:
  [CONTRIBUTING.md#toolchain](CONTRIBUTING.md#toolchain).
- Verification commands: [CONTRIBUTING.md#build-and-test](CONTRIBUTING.md#build-and-test).
- `tests/ui/` layout, blessing, `checker-args`/`checker-env`, new case per
  rule change: [CONTRIBUTING.md#regression-tests](CONTRIBUTING.md#regression-tests).
- Measured iterator numbers to re-run after traversal changes:
  [CONTRIBUTING.md#iterator-measurements](CONTRIBUTING.md#iterator-measurements).
- Why `cargo no-alloc` never runs `cargo check`:
  [CONTRIBUTING.md#manual-verification](CONTRIBUTING.md#manual-verification).
- Architecture: [docs/design.md](docs/design.md); scope of the guarantee:
  [docs/guarantees.md](docs/guarantees.md).

## Agent-only rules

- Never run `cargo publish`, `cargo release`, the release workflow, or any
  other publishing step unless the user explicitly asks for it. Publishing is
  permanent; the procedure is in [docs/releasing.md](docs/releasing.md).
