# Design

The brief, decisions, milestone plan, and required test matrix live in the
planning document this repo was built from; this file records what
*implementation* confirmed or changed, so it doesn't drift from reality the
way a restated brief would. See the `adr/` directory for the load-bearing
decisions and their rationale, and `README.md`/`AGENTS.md` for usage and
working notes.

## Toolchain

Pinned via `rust-toolchain.toml`: `nightly-2026-08-01` (`rustc 1.99.0-nightly
(ad3d0bc14 2026-07-31)`, LLVM 22.1.8), components `rustc-dev`, `rust-src`,
`llvm-tools-preview`.

`rustc-dev` ships the **full compiler source** at
`<toolchain>/lib/rustlib/rustc-src/rust/compiler/`, not just the compiled
rlibs. Every rustc-internals API used by this tool was verified by grepping
that tree rather than by trusting memory of a different nightly's API — the
surface changes release to release and the source is the only ground truth
that matches the pinned commit exactly. `rust-src` (the other component)
only ships `library/` (std/core/alloc); it does **not** include `compiler/`.

## M1 findings (driver skeleton)

- `rustc_driver::{Callbacks, Compilation, run_compiler, catch_with_exit_code}`
  match the shape a recent rustc driver tool (clippy/miri-style) expects:
  `Callbacks::after_analysis<'tcx>(&mut self, &interface::Compiler,
  TyCtxt<'tcx>) -> Compilation`, `run_compiler(&[String], &mut dyn Callbacks)`
  (strips `argv[0]` internally), `catch_with_exit_code(FnOnce() -> T) ->
  ExitCode`.
- The standard cargo-wrapper argv convention holds: under
  `RUSTC_WORKSPACE_WRAPPER`, cargo invokes
  `no-alloc-driver <real-rustc-path> <rustc args...>`; the driver strips
  `args[1]` when its file stem is `rustc` (same check clippy-driver uses),
  so direct invocation as `RUSTC=no-alloc-driver` also works without that
  element present.
- `-Zcrate-attr`, `-Zalways-encode-mir`, and `feature(register_tool)` all
  exist at this nightly (grepped `rustc_session/src/options.rs` and
  `rustc_feature/src/unstable.rs`).
- `build.rs`'s `cargo::rustc-link-arg-bin=...-Wl,-rpath,{sysroot}/lib` works:
  confirmed via `readelf -d` (`RUNPATH` set to the toolchain's `lib/` dir)
  and by running the driver binary directly with `LD_LIBRARY_PATH` cleared.
- End-to-end confirmed against a toy crate: `RUSTC_WORKSPACE_WRAPPER` only
  applies to the workspace-member crate (`toy_m1`), not to its path
  dependencies (`no_alloc`, `no_alloc_macros`) — they compiled without the
  driver's log lines, exactly as the `--target <host>` split is supposed to
  produce.

## M2 findings (mono graph dump)

- `tcx.collect_and_partition_mono_items(())` returns `MonoItemPartitions<'tcx>`
  (a struct, not a tuple — resolves the plan's open item):
  `{ codegen_units: &'tcx [CodegenUnit<'tcx>], all_mono_items: &'tcx DefIdSet }`.
  `all_mono_items` is DefId-deduplicated and **not** what per-instantiation
  analysis wants; the per-instantiation enumeration is
  `cgu.items(): &FxIndexMap<MonoItem<'tcx>, MonoItemData>` on each codegen
  unit, filtered to `MonoItem::Fn(instance)`. Both live in
  `rustc_middle::mono` (crate-root `mono` module, not under `mir`).
- `Instance` and `MonoItem` both implement `Display` via `FmtPrinter`,
  giving readable output with concrete substitutions for free
  (`identity::<i32>`, `identity::<f64>`, `<En as Greet>::greet`, ...).
- Confirmed against a toy crate with a two-instantiation generic fn and a
  `dyn Trait` call: both `identity::<i32>` and `identity::<f64>` appear as
  distinct `MonoItem::Fn` entries, and the concrete `<En as Greet>::greet`
  impl appears too (it must exist for the vtable regardless of the call
  being indirect — the traversal at M4 still has to treat the *call site*
  as unresolved, since resolving the mono item that backs a vtable slot is
  not the same as resolving what a given virtual call dispatches to).
- **Root attribute lookup**: `TyCtxt::get_attrs_by_path(def_id, &[Symbol;
  N])` (in `rustc_middle::ty`, not deprecated, unlike `get_attrs`/
  `get_all_attrs`) works for both local and foreign `DefId`s — it branches
  internally on `def_id.as_local()`, calling `hir_attrs` for local and the
  `attrs_for_def` query for foreign. `#[no_alloc_tool::root]` is a tool
  attribute, so it lands as `hir::Attribute::Unparsed`, matched via
  `path_matches(&[sym::no_alloc_tool, sym::root])`.
- **Cross-crate metadata survival, resolved by evidence**: confirmed with a
  two-crate fixture (`toy_m2_dep` lib exporting `#[no_alloc::no_alloc] pub
  fn dep_root()`, `toy_m2_leaf` bin depending on it, both compiled under the
  checker driver as one cargo workspace). The leaf's `after_analysis` walked
  `tcx.crates(())` → `tcx.module_children(cnum.as_def_id())` → found
  `dep_root`'s foreign `DefId` → `get_attrs_by_path` returned the attribute.
  Log line: `foreign root attribute visible via cross-crate metadata
  crate_name=toy_m2_dep def_path=toy_m2_dep::dep_root`. See
  [ADR 0002](../adr/0002-cfg-gated-tool-attribute-marker.md) for what this
  means for the sidecar index (belt, not mechanism).
- `rustc --print host-tuple` is the current print-request key for the host
  triple (`rustc_session/src/config/print_request.rs`); used by
  `cargo-no-alloc` to pass `--target <host>` explicitly.

## What M2 did not need to answer yet

`-Zbuild-std` default and full std MIR coverage on the stock sysroot are
M3 questions (leaf predicate + traversal need real allocator-path frames to
test against, which the M2 toy crates don't exercise). `exchange_malloc`
lang-item presence is also M3.

## M3 findings (leaf predicate)

- `library/alloc/src/alloc.rs`, confirmed verbatim: `__rust_alloc`,
  `__rust_dealloc`, `__rust_realloc`, `__rust_alloc_zeroed` carry
  `#[rustc_allocator]`, `#[rustc_deallocator]`, `#[rustc_reallocator]`,
  `#[rustc_allocator_zeroed]` respectively, inside an `unsafe extern "Rust"
  { ... }` block. That block matters: these are foreign declarations with
  **no MIR body** in this crate (the real body is generated later, by a
  `#[global_allocator]` shim or std's `__rdl_alloc`/`__rdl_alloc_zeroed`).
  A traversal that checks "no MIR body → reject" before checking "is this
  an allocator terminal" would silently misclassify a real violation as a
  rejection. `leaf::allocates` is written and ordered accordingly, and this
  ordering is proven, not just documented: see the probe result below,
  where the terminal frame has `has_mir=false` and is still correctly
  reported as the allocator terminal.
- `exchange_malloc` **is confirmed gone**: no hits anywhere in the pinned
  nightly's compiler source or `library/`. Only `owned_box` remains as a
  lang item (`Box<T>`'s type marker, not an allocation call site). No
  lang-item fallback is needed in the leaf set; the `codegen_fn_attrs` flag
  path covers `Box::new` on its own.
- `CodegenFnAttrFlags::{ALLOCATOR, DEALLOCATOR, REALLOCATOR,
  ALLOCATOR_ZEROED}` all exist as documented. Checking all four, not just
  `ALLOCATOR`, is not belt-and-suspenders — it's load-bearing: the M3 probe
  (below) shows `Box::new(5i32)` resolving to `__rust_alloc_zeroed`, not
  `__rust_alloc`. A leaf predicate checking only `ALLOCATOR` would have
  missed this real path.
- **New API detail, not in the original brief**: a `Call`/`TailCall`
  callee's `ty::FnDef(def_id, args)` carries `args` wrapped in a `Binder`
  in this nightly (confirmed against `rustc_monomorphize/src/collector.rs`,
  which calls `args.no_bound_vars().unwrap()` before resolving). M4's
  traversal must call `.no_bound_vars()` and — unlike the M3 probe, which
  just skips the edge — treat `None` (unresolved bound vars) as a
  **rejection**, not a silent skip, per ADR 0003: a callee whose args can't
  be pinned down statically is exactly the "no statically available body"
  case that ADR exists to cover.
- **Probe result** (`no_alloc_analysis::probe::debug_probe_chain`, a BFS
  over `Call`/`TailCall` edges only — no Drop, no reject/continue
  distinction, explicitly not the M4 traversal, run against a toy crate
  with `#[no_alloc] fn root() { let b = Box::new(5i32); *b }`):

  ```
  root → std::boxed::Box::<i32>::new → std::boxed::box_new_uninit
    → std::alloc::Global::alloc_impl_runtime → alloc::alloc::__rust_alloc_zeroed
  probe_chain terminated: allocator, chain_len=5, mir_available=5, mir_missing=3
  ```

  Every frame up to and including the point where MIR stops mattering had
  MIR available on the **stock sysroot** (no `-Zbuild-std`); the only
  missing-MIR frame *in the winning chain* is the terminal itself, which is
  expected (it's the `extern "Rust"` declaration, per above) rather than a
  coverage gap.
- **`-Zbuild-std` decision: off by default.** The stock sysroot has MIR for
  every real frame on this allocation path. Revisit only if a later UI test
  fixture (M4 test matrix) hits a std frame the stock sysroot lacks MIR
  for; nothing found at M3 requires it.

## M4 findings (traversal)

Root-instance collection for M4 covers the whole required test matrix
without needing sidecar file I/O: local attribute-based roots ∪
`NO_ALLOC_ROOTS` env matches, instantiated by scanning the crate's own mono
graph for `MonoItem::Fn` sharing the root's `DefId` (mono-site: a generic
root gets one verdict per instantiation actually present), falling back to
`Instance::mono` for a local non-generic root that was never called.
Cross-crate root aggregation (reading another workspace crate's sidecar, or
walking a dependency's foreign roots into the analysis rather than just the
M2 probe) is **not wired up** — no required test case is multi-crate, and
direct cross-crate attribute reading is already proven working (M2) if this
is needed later.

The full 13-fixture matrix (the 10 required rows plus mutual recursion, a
`dyn` call in a dead branch, and a `Vec` drop in one arm only) was run
end-to-end through the real driver and matched every expected outcome —
but not on the first pass. Two real bugs surfaced, both exactly the kind
the matrix exists to catch:

1. **Violation-vs-rejection priority within one function.** MIR splits a
   function into one basic block per `Call`, even for straight-line code
   with zero branching — e.g. `alloc::alloc::alloc` calls the no-op
   stability-shim `__rust_no_alloc_shim_is_unstable_v2()` *before*
   `__rust_alloc(...)`, as two sequential (not alternative) calls. The
   first traversal returned on whichever basic block's terminator it
   classified first in iteration order — which was the shim call — so
   `direct_alloc` (plain `Box::new`) came back **REJECTED** ("no MIR
   body"), not **VIOLATION**, because the shim itself has no body and
   isn't an allocator. This would have made the tool's single most basic
   case report the wrong verdict category, with a misleading reason
   string, while looking superficially like it "worked" (rejection isn't
   unsound, just wrong). Fixed: `visit` now explores every edge of a
   function before deciding, and violation found anywhere beats rejection
   — see the doc comment on `no_alloc_analysis::traversal` for the full
   writeup. `Finding` chains are materialized (cloned) the instant they're
   discovered rather than reconstructed later from a shared mutable stack,
   which is what makes "keep searching after finding a rejection" safe.
2. **ICE on `InstanceKind::Intrinsic`/`LlvmIntrinsic`.** `tcx.instance_mir`
   panics ("intrinsics have no instance MIR") if called with one of these
   two instance kinds — they don't just lack `is_mir_available`, calling
   the query is itself invalid. The `drop_in_one_arm` fixture (`Vec::push`
   triggering a real intrinsic internally) crashed the driver until the
   "no callable MIR" guard checked `instance.def`'s kind
   (`Virtual`/`Intrinsic`/`LlvmIntrinsic`, matching the trio the compiler's
   own doc comments on `InstanceKind` group together) *before*
   `is_mir_available`/`is_foreign_item`, not after.

Result after both fixes, all 13 fixtures against the real driver:

| Fixture | Verdict | Matches required row |
|---|---|---|
| `pure_arith` | Pass | trivial false positive |
| `direct_alloc` | Violation: `root → Box::<T>::new → box_new_uninit → Global::alloc_impl_runtime → __rust_alloc_zeroed` | leaf set wired up |
| `generic_split` | `<AllocKind>` Violation, `<PureKind>` Pass | mono-site justification |
| `drop_field` | Violation via 4×`drop_glue` → `RawVec::drop` → `RawVecInner::deallocate` → `Global::deallocate` → `__rust_dealloc` | Drop edge not missing |
| `dyn_reject` | Rejected (`Greet::greet` resolves to `InstanceKind::Virtual`) | dispatch rule |
| `fnptr_reject` | Rejected ("callee is a function pointer") | dispatch rule, non-`dyn` form |
| `extern_reject` | Rejected (`abs`, no MIR) | no-body rule |
| `inline_asm_reject` | Rejected ("inline assembly...") | opaque terminator |
| `recursive_pass` | Pass | memoization/termination |
| `global_allocator` | Violation, identical chain to `direct_alloc` | leaf set not bypassable |
| `dead_branch_dyn` | Rejected even though `root(false)` never takes that branch at runtime | non-path-sensitivity |
| `mutual_recursion` | Pass | cross-function cycle, not just self-recursion |
| `drop_in_one_arm` | Violation via `Vec::push → ... → __rust_alloc_zeroed` | branch-local violation still found; also the Intrinsic-guard regression case |

Also notable: `Box::new(5i32)` and `Vec::push`'s growth both route through
`__rust_alloc_zeroed`, not `__rust_alloc` — consistent with the M3 finding
that all four `codegen_fn_attrs` flags are load-bearing, not just
`ALLOCATOR`.

`dyn` dispatch was expected (per the original brief) to show up as a
non-`FnDef` (`FnPtr`) callee type at the MIR level, sharing one rejection
branch with real fn pointers. In practice `Greet::greet` on a `&dyn Greet`
receiver *is* a `FnDef` callee (the trait method's own `DefId`), and
`Instance::try_resolve` resolves it successfully to an
`InstanceKind::Virtual` instance rather than returning `Ok(None)`/`Err`.
Both forms still end up rejected, just via the two different guards (the
`InstanceKind::Virtual` check, not the "non-`FnDef` callee" branch) — this
doesn't change any verdict, only which code path produces it, but the
`Rejected` reason string doesn't yet distinguish "vtable dispatch" from
"any other no-MIR callee"; sharpening that is left to M5's diagnostics
pass, not a correctness gap.

## M5 findings (diagnostics)

- `TyCtxt::dcx()`, `DiagCtxtHandle::{struct_span_err, struct_span_warn}`,
  and `Diag::span_note` all match the plan's description exactly.
  `struct_span_err`/`struct_span_warn` return `Diag<'a, ErrorGuaranteed>`
  vs. `Diag<'a, ()>` — different generic params, so `warn_only` is a
  branch that builds two separate diagnostics (each still doing the same
  `span_note` loop), not a single shared `Diag` value.
- **Span provenance was the actual design work here, not just plumbing.**
  Each `Frame`'s span is the *call/drop site* in the caller's body
  (`terminator.source_info.span`), not the callee's `def_span` — that's
  what makes the diagnostic chain point at the specific `Box::new(5i32)`
  line rather than just every callee's function signature. The root frame
  alone uses `tcx.def_span(root.def_id())` (nothing called it). Rendered
  via `tcx.sess.source_map().span_to_diagnostic_string(span)`.
  `no_alloc_report::Frame::span` stores the *rendered string*, not a real
  `Span` (that crate has no rustc dependency), so `traversal::check_instance`
  returns a `Checked` wrapper carrying both the stable `Verdict` (for
  `report.json`/tests) and the raw `Vec<(Instance, Span)>` (for
  `diagnostics::emit`, which needs real spans to call `dcx()` with).
- Confirmed end-to-end against the `direct_alloc` fixture: the rendered
  diagnostic's `note: via` chain includes real `library/alloc/src/...`
  spans from the installed `rust-src`, not just def paths — genuinely
  actionable, not a bare "this allocates". `report.json`'s chain carries
  the same spans, machine-rendered.
- `NO_ALLOC_WARN_ONLY=1` confirmed: same diagnostic content, `warning:`
  instead of `error:`, build still succeeds (`Finished`, exit 0). Without
  it, the same crate fails with exit 101 and no binary is produced —
  `after_analysis` returns `Compilation::Stop` on any hard error, skipping
  codegen entirely (no point codegening a crate whose check already
  failed).
- Root-instance collection landed as designed: local attribute-based roots
  ∪ `NO_ALLOC_ROOTS` env matches, instantiated via the mono graph (mono-site:
  `generic_split`'s two instantiations get independent verdicts), falling
  back to `Instance::mono` for an uncalled non-generic root. Sidecar
  read/union (cross-workspace-crate roots) is **not implemented** — no
  required test is multi-crate, and M2 already proved direct cross-crate
  attribute reading works if this is needed later; the `no_alloc_report`
  sidecar types exist and are unit-tested, but nothing writes or reads a
  sidecar file yet. Flagging this explicitly so it doesn't read as an
  oversight.

## The regression suite (`tests/ui/`)

13 fixtures (the 10 required rows plus mutual recursion, `dyn` in a dead
branch, and a `Vec` drop in one arm only), each a self-contained crate
(own `[workspace]` — necessary so cargo doesn't try to fold it into the
outer workspace when `cargo-no-alloc` runs with `cwd` inside it) under
`tests/ui/<case>/`, driven by `tests/ui.rs` (the workspace root doubles as
a package — `no_alloc_harness` — precisely so `tests/`/`benches/` can sit
at the repo root rather than inside one crate). Two assertions per case:

- `expected.json`: primary, portable. A normalized `report.json`
  projection with spans stripped — a rendered span embeds an absolute
  toolchain path (`/home/dw/.rustup/.../boxed.rs:...`), which isn't
  portable across machines or even across `rustup` install locations on
  the same machine. The `def_path` chain + verdict kind + reject reason is
  the part a real regression would actually change.
- `expected.stderr`: a snapshot of the rendered diagnostic, spans and all.
  Explicitly not claimed to be portable — re-bless with `NO_ALLOC_BLESS=1`
  when the toolchain or its install path changes. Getting a clean,
  reproducible snapshot required two fixes: setting `NO_ALLOC_LOG=off` on
  the child process (our own `tracing` output has real timestamps, which
  are never reproducible), and filtering out cargo's own build-progress
  lines (`Compiling`/`Updating crates.io index`/`Locking N packages`),
  which vary with local registry cache state and have nothing to do with
  this tool's output.
- The harness also asserts the process exit code matches
  `Report::is_success()` — confirms `cargo-no-alloc` actually propagates
  driver failure through cargo's own exit status, not just that the JSON
  looks right.

Binary discovery (`cargo-no-alloc`/`no-alloc-driver`) is via
`CARGO_MANIFEST_DIR`/`target/<profile>/`, not a `current_exe()`
sibling-walk: this sandbox places test/bench binaries under
`target/<profile>/build/<pkg>/<hash>/out/`, not the conventional
`target/<profile>/deps/`, which breaks the "walk up two directories" trick
`cargo-no-alloc` itself uses to find the driver (that one still works
because it's a real installed/built binary locating a *sibling* binary in
the same directory, not a test harness locating anything).

## Benchmarks

`benches/report.rs` (criterion, `harness = false`): `parse_root_spec` and
`Report` serde round-trips over a 64-frame chain — the pure parts, on
stable-compatible code.

`benches/throughput.rs` (wall-clock, `harness = false`, not criterion):
runs the real `cargo-no-alloc` against `benches/fixtures/throughput/`, a
generated 1365-function crate (depth-5, branching-4 call tree, all pure
arithmetic — deliberately non-violating, since the DFS's
violation-wins-early-exit would otherwise short-circuit before visiting
the whole graph, defeating the point of a throughput measurement).
Touches the fixture source to force re-analysis (cargo would otherwise see
nothing changed and skip invoking the driver), then times a single build.
Observed: ~6,700 instances/sec on this machine, debug-profile driver (the
bench harness itself defaults to `release`, but the driver is normally
only built `debug`; the bench prefers a release driver if one happens to
exist, otherwise uses debug rather than forcing an extra build the
documented workflow doesn't ask for).

## Coverage

`cargo llvm-cov --workspace --summary-only -- --skip ui_matrix`. The
`--skip ui_matrix` is necessary, not cosmetic: `tests/ui.rs` exercises
`no_alloc_analysis`/`no_alloc_driver` by spawning the **built, uninstrumented**
`cargo-no-alloc` binary as a subprocess — llvm-cov instruments the crates
linked into the *test binary itself*, and a subprocess invocation of a
separately-built binary is invisible to it. (Running it anyway under
`cargo llvm-cov` — before this was discovered — produced a spurious
`inline_asm_reject` snapshot mismatch tied to llvm-cov's alternate
`--target-dir`; root cause not fully chased down since the test provides
zero coverage signal under instrumentation regardless.) Real coverage
scope is exactly what the plan's own wording says: "the pure crates and
the harness" — `no_alloc_report` and `no_alloc_macros` land at 97–100%;
`no_alloc_analysis` doesn't appear in the report at all (`test = false`,
by design — see the M0 note on rlib/dylib linking); `cargo_no_alloc`/
`no_alloc_driver` show 0% from this command specifically because their
real exercise is subprocess-based, not because they're untested — the
full `tests/ui.rs` run (13/13 passing, `cargo test --workspace`, no
`--skip`) is the actual evidence for that code.

## Status

M0–M5 complete and observed against the real toolchain: driver, analysis,
diagnostics, the full required test matrix (13 fixtures, `tests/ui.rs`,
both `expected.json` and `expected.stderr` assertions), macro unit tests
(trybuild), benchmarks (criterion + wall-clock throughput), and coverage
all built and passing. `cargo build --workspace`, `cargo test --workspace`,
`cargo fmt --all --check`, and `cargo clippy --workspace --all-targets` are
all clean. Known, deliberately-scoped gaps: sidecar root-index read/union
across workspace crates (not needed by any required test; M2 already
proved the alternative — direct cross-crate attribute reading — works),
and `-Zbuild-std` (measured off at M3, no fixture has since needed it).
