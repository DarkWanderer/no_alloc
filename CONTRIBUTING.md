# Contributing

## Toolchain

The checker and its rustc-internals crates (`rustc_middle`, `rustc_interface`,
...) are pinned to `nightly-2026-08-01` in `rust-toolchain.toml`, and the
tool is Linux-only. **Do not bump the pinned nightly casually** — those
crates have no stability guarantee across nightlies, and the driver's API
usage is verified against this exact version. See `AGENTS.md` and
[ADR 0001](adr/0001-mono-site-analysis.md) for why the analysis is written
this way in the first place.

```bash
rustup toolchain install nightly-2026-08-01 --component rustc-dev,rust-src,llvm-tools-preview
```

`no_alloc_report` and `no_alloc_check` are the exception: they have no
`rustc_private` dependency and must stay buildable and testable on stable.
Keep rustc-internals types out of them; convert at the boundary in
`no_alloc_analysis`.

## Build and test

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo llvm-cov --workspace --summary-only -- --skip ui_matrix
cargo bench
cargo audit
```

`no_alloc_analysis` sets `test = false` in `[lib]` (see its `Cargo.toml`):
the rustc-private sysroot crates it depends on ship dylib+rmeta only, no
rlib, so Cargo's default unittest harness fails to link it even with zero
`#[test]` functions. Its logic is exercised end-to-end by `tests/ui.rs`
instead, driving the built `cargo-no-alloc`/`no-alloc-driver` binaries
against fixtures. Note that `cargo test --workspace <filter>` can still
attempt to build and link it despite `test = false` — if you hit link errors
running a filtered workspace test, that's why; run the fixture crate
un-filtered or via `cargo +stable test -p no_alloc_check -p no_alloc_report`
for the stable-only subset instead.

## Regression tests

Each case under `tests/ui/<case>/` is a full toy crate, not a snippet. A case
has:

- `expected.json` — asserted against the checker's report; this is the
  actual test.
- `expected.stderr` — a snapshot of compiler/checker stderr, re-blessed with
  `NO_ALLOC_BLESS=1 cargo test ...` rather than hand-edited.

**Adding an edge-table rule or leaf-set change should come with a new case
under `tests/ui/`, not just a new assertion bolted onto an existing case.**
Existing cases are named for the one thing they exercise (`dyn_reject`,
`drop_field`, `panic_abort`, ...); keep that convention.

## Soundness

Any change to `crates/no_alloc_analysis` must preserve "reject, don't
assume" for unresolved call edges — see
[ADR 0003](adr/0003-reject-unresolved-edges.md). Do not special-case an edge
kind into "assume safe" to make a test pass; if a rejection looks wrong,
either it's a real bug to fix soundly, or it's a documented scope exclusion
(see README's "Guarantee and limitations") that belongs in the docs, not in
a special case.

Analysis runs on the monomorphized instance graph
(`collect_and_partition_mono_items`), not on function definitions — see
[ADR 0001](adr/0001-mono-site-analysis.md). A verdict is per-instantiation;
do not memoize or report results by `DefId` alone.

## Manual verification

`cargo no-alloc` invokes `cargo build`/`cargo test`, never `cargo check` —
the monomorphized graph does not populate without reaching codegen. Every
run also runs `cargo clean` first (see `docs/design.md`), so testing it
manually against a fixture is always a full rebuild, not an incremental one.

## License

By contributing, you agree that your contributions are licensed under either
[MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at the user's option,
matching the rest of the project.
