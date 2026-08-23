# no_alloc — working notes for coding agents

- This tool makes a soundness claim (no allocator reachable from a marked
  root). Any change to `crates/no_alloc_analysis` must preserve "reject,
  don't assume" for unresolved call edges — see
  [ADR 0003](adr/0003-reject-unresolved-edges.md). Do not special-case an
  edge kind into "assume safe" to make a test pass.
- Analysis runs on the **monomorphized instance graph**
  (`collect_and_partition_mono_items`), not on function definitions. See
  [ADR 0001](adr/0001-mono-site-analysis.md). A verdict is per-instantiation;
  do not memoize or report by `DefId` alone.
- The pinned nightly is `nightly-2026-08-01`. Do not bump it casually — the
  rustc-internals crates (`rustc_middle`, `rustc_interface`, ...) have no
  stability guarantee across nightlies, and the driver's API usage is
  verified against this exact version.
- `no_alloc_report` must stay buildable and testable on stable — it has no
  `rustc_private` dependency. Keep rustc-internals types out of it; convert
  at the boundary in `no_alloc_analysis`.
- Regression tests live in `tests/ui/<case>/`. Each case is a full toy crate
  plus `expected.json` (assert on this) and `expected.stderr` (snapshot,
  `NO_ALLOC_BLESS=1` to update). Adding an edge-table rule or leaf-set change
  should come with a new case, not just an assertion on an existing one.
- `cargo no-alloc` invokes `cargo build`/`cargo test`, never `cargo check` —
  the mono graph does not populate without reaching codegen.
