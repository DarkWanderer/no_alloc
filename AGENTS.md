# no_alloc — working notes for coding agents

- This tool makes a soundness claim (no allocator reachable from a marked
  root). Any change to `crates/no_alloc_analysis` must preserve "reject,
  don't assume" for unresolved call edges — see
  [ADR 0003](adr/0003-reject-unresolved-edges.md). Do not special-case an
  edge kind into "assume safe" to make a test pass.
- The non-allocating intrinsic table
  (`crates/no_alloc_report/src/intrinsic_table.rs`) is an allowlist and a
  soundness surface — see [ADR 0005](adr/0005-intrinsic-leaf-classification.md).
  Adding a name asserts that the compiler's lowering of that intrinsic
  cannot call into Rust code; an intrinsic that runs a caller-supplied
  function (`catch_unwind`, `const_eval_select`, the contract checks) must
  never be added, and neither must the validity assertions
  (`assert_inhabited` and friends), whose codegen emits a panic call for a
  failing instantiation — those are classified per instantiation in
  `traversal.rs` instead. Unknown names reject, which is the correct default
  — do not "complete a family" to make something pass.
- Iterators are only checkable under `cargo no-alloc --immediate-abort`
  ([ADR 0006](adr/0006-immediate-abort-checking-mode.md)); `docs/iterators.md`
  records what passes, measured from `examples/iterators`. If you change the
  traversal, re-run that example and update the numbers in the doc, its
  README, and `examples/iterators/src/main.rs` — they are stated as measured
  facts, not estimates.
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
  `NO_ALLOC_BLESS=1` to update), and optionally a `checker-args` file for a
  case that needs non-default checker flags. Adding an edge-table rule or a
  leaf-set change should come with a new case, not just an assertion on an
  existing one.
- `cargo no-alloc` invokes `cargo build`/`cargo test`, never `cargo check` —
  the mono graph does not populate without reaching codegen.
