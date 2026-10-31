# no_alloc

[![CI](https://github.com/DarkWanderer/no_alloc/actions/workflows/ci.yml/badge.svg)](https://github.com/DarkWanderer/no_alloc/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/cargo-no-alloc.svg)](https://crates.io/crates/cargo-no-alloc)
[![docs.rs](https://docs.rs/no_alloc_report/badge.svg)](https://docs.rs/no_alloc_report)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

`no_alloc` statically checks that selected Rust function instances cannot
reach the global allocator. It is intentionally conservative: unresolved
calls are rejected instead of assumed safe.

The checker is currently Linux-only and tied to `nightly-2026-08-01`. One
consequence: `no_alloc_analysis` requires `#![feature(rustc_private)]` and
cannot build on docs.rs, so the docs.rs badge above points at
`no_alloc_report` — the stable-compatible report crate — instead.

## Install and use

Install the marker macro first, then the Cargo command:

```bash
cargo add no_alloc_check@0.1.0
rustup toolchain install nightly-2026-08-01 --component rustc-dev,rust-src
cargo +nightly-2026-08-01 install cargo-no-alloc --version 0.1.0
```

Mark functions and concrete methods with the stable-compatible attribute:

```rust
#[no_alloc_check::no_alloc]
fn process(sample: f32) -> f32 {
    sample * 0.5
}
```

The macro rejects async functions, trait methods without bodies, and
non-function items because their executable body cannot yet be selected
soundly. Add this lint configuration to consumers:

```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(no_alloc_check)'] }
```

Run the checker with Cargo arguments after `--`:

```text
cargo no-alloc [--all-crates] [--build-std] [--immediate-abort]
               [--warn-only] [--root PATH]... -- [build|test] [CARGO_ARGS...]
```

`build` is the default. `check` is rejected because it does not produce the
monomorphized graph. `test` is accepted but is not a supported mode for
meaningful results — see "Guarantee and limitations" below. The checker owns
its target and target directory, so user-supplied `--target` and
`--target-dir` are rejected. `--root` selects an unannotated function by
canonical path across the complete build; it accepts a comma-separated list
and may be repeated (`--root a,b --root c`). A malformed command line, such
as `--root a,,b`, is an argument error and writes no report.

By default only workspace members are instrumented (`RUSTC_WORKSPACE_WRAPPER`).
A `#[no_alloc]` marker on a non-generic function in a registry or path
dependency outside the workspace is therefore silently never analyzed — the
function never appears in the report at all, which is different from an
unmatched `--root` (reported as a selection error). A marked *generic*
function is checked wherever a workspace crate's mono graph reaches an
instantiation, including one a dependency already emitted and the workspace
crate links to under share-generics
([ADR 0004](adr/0004-cross-crate-roots-and-report-aggregation.md)). Pass
`--all-crates` to instrument every crate in the build (`RUSTC_WRAPPER`)
instead.

Unless `RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS` is set, the checked build uses
the `build.rustflags` / `target.<host>.rustflags` from your Cargo config, as a
plain `cargo build` would; `target.'cfg(...)'.rustflags` is not supported and
the run is refused (put those flags in `RUSTFLAGS`). A `rustc-wrapper` or
`rustc-workspace-wrapper` in Cargo config (or its `CARGO_BUILD_*` environment
form) is ignored for the run with a warning, because the driver must be the
only wrapper; the `RUSTC_WRAPPER` and `RUSTC_WORKSPACE_WRAPPER` environment
variables are refused. The checker compiles with the `rustc` found on `PATH`,
so a configured `build.rustc` is refused (a substitute compiler could add
`--cfg` flags the checked build would not see), as are the `RUSTC` and
`CARGO_BUILD_RUSTC` variables. A configured
`build.target` (`[build] target`, `CARGO_BUILD_TARGET`, `--config`) is
refused, since the checker always builds for the host; so is a workspace
member's `default-target` / `forced-target` (`per-package-target`). Keys starting
`NO_ALLOC_` in Cargo's `[env]` table are refused.

A non-UTF-8 `RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS` is refused. These
configuration refusals (rustc override, `RUSTC_WRAPPER`, ambient
immediate-abort without `--build-std`, non-UTF-8 flags) persist a failing
`report.json`; operational errors and argument errors do not.

The existing `NO_ALLOC_ROOTS`, `NO_ALLOC_WARN_ONLY`, and `NO_ALLOC_LOG`
environment interfaces remain supported. The final deterministic report is
written to `target/no-alloc/report.json`. Its `warnings` list (omitted when
empty) holds conditions that leave the exit code at 0 but that a reader of the
file must not mistake for a verified pass; today that is a run that checked
zero roots, including one where every root was `not_instantiated`. Each checked root carries a versioned
`environment` block with its typed panic strategy, optimization and MIR
optimization levels, target triple, rustc version, and the
`--all-crates`/`--build-std` settings. These values matter to what was proven,
and recording them per root preserves the real configuration when one Cargo
build contains both host and target artifacts. The report-level
`panic_strategy` remains for compatibility and is omitted when those artifacts
disagree.

Every invocation runs `cargo clean` on the checker's target first, so
**every `cargo no-alloc` run is a from-scratch rebuild**, never an
incremental one. This is deliberate — it guarantees that a checker
configuration change can never be hidden by Cargo's build cache (see
[`docs/design.md`](docs/design.md)) — but it means every run pays full
compile time, not just the cost of what changed.

`--immediate-abort` rebuilds the crate and the standard library with
`-Cpanic=immediate-abort` (implying `--build-std`, and incompatible with
`-- test`, which rustc will not build under an abort strategy), which is
what makes iterator-shaped code checkable at all — see
[`docs/iterators.md`](docs/iterators.md) and
[ADR 0006](adr/0006-immediate-abort-checking-mode.md).

See [`examples/basic`](examples/basic) for a runnable example, and
[`examples/iterators`](examples/iterators) for the 35 iterator patterns
behind `docs/iterators.md`.

## Guarantee and limitations

Analysis starts from each selected monomorphized `Instance`, follows resolved
calls, tail calls, compiler-generated shims, and drop glue, and detects
allocator, deallocator, reallocator, and zeroed-allocator terminals.
Function pointers, virtual dispatch, foreign calls without MIR, inline
assembly, and intrinsics outside the non-allocating intrinsic table are
rejected ([ADR 0005](adr/0005-intrinsic-leaf-classification.md),
[ADR 0007](adr/0007-shim-and-fn-item-resolution.md)). A
`Violation` chain names *a* reachable allocator path, not necessarily the one
actually taken at runtime — the DFS reports whichever call sequence it finds
first among several that may exist; this is correct for a reachability
claim, but don't read the chain as "this is the line that allocates" without
checking.

A `mem::swap` of a non-SSA type in a local root can be rejected when a linked
dependency built at the same opt-level instantiates the same `mem::swap::<T>`
(share-generics makes it upstream-shared); this is a conservative false
rejection ([ADR 0005](adr/0005-intrinsic-leaf-classification.md)).

**The panic exclusion is drawn by terminator shape, not by whether a path
panics.**
`UnwindResume` and `UnwindTerminate` have no MIR callee to follow, so the
traversal treats them as terminal control flow rather than calls — but each
lowers to a real call during codegen (a foreign `_Unwind_Resume`, or a
nounwind panic-handler call) that the traversal never walks. `Unreachable`
codegens to a bare LLVM `unreachable` and contains no code at all. An
`Assert` terminator (the implicit checks inserted for array bounds,
arithmetic overflow, and similar cases) is allowed only under a non-unwinding
panic strategy (`panic=abort`), and that is *not* because the panic runtime
is unreachable there: a failing assertion under plain `panic=abort` still
calls `panic_fmt` and the `#[panic_handler]` (which, with `std`, formats and
prints — i.e. allocates) before the process aborts. It is allowed because
allocation reachable only through a panic, an unwind-terminate, or an
unwind-resume path is out of scope for this guarantee, not because that
allocation has been ruled out. An explicit `panic!()`, `.unwrap()`, or `.expect()` has a
different MIR shape: it is an ordinary `Call` terminator and remains in scope,
so an unresolved panic-runtime callee rejects even under `panic=abort`. Under
`panic=unwind`, `Assert` is rejected outright instead, on the usual
reject-don't-assume grounds — see [ADR 0003](adr/0003-reject-unresolved-edges.md)
for the full terminator classification and rationale.

**`--immediate-abort` is the exception, and the stronger guarantee.** It
compiles the crate and the standard library with `-Cpanic=immediate-abort`,
where every panic — including the panic lang item a failing `Assert` calls —
lowers to a bare `abort()` with no handler call. The traversal then walks
panic paths, `Assert`'s included, as ordinary edges and
finds `core::intrinsics::abort` at the end of them, so in that mode there is
no panic-path carve-out at all. It costs a full sysroot rebuild per run. See
[ADR 0006](adr/0006-immediate-abort-checking-mode.md).

In practice this means most indexing and arithmetic in debug builds — which
lower to an `Assert` — reject under `panic=unwind` (Rust's default), and
`panic = "abort"` in `[profile.*]` is the minimum for anything to pass. It is
not enough for iterators: the standard library reaches the panic machinery
through ordinary calls as well as `Assert` terminators, and those reject
until the sysroot is rebuilt. Measured over 35 iterator patterns, `abort`
passes none of them and `--immediate-abort` passes 33 — see
[`docs/iterators.md`](docs/iterators.md).

**The boundary is MIR, not the machine code.** The analysis follows the
calls the compiler puts in MIR; it does not see the ones the backend
synthesizes from it. A large struct move becomes a `memcpy` call, `f32::powf`
becomes a `powf` call, `f128` arithmetic becomes a `compiler_builtins` call —
none of which appear as MIR terminators. Those symbols are assumed to be the
platform's. A program that defines one of them itself, in Rust, and
allocates there (`#[no_mangle] extern "C" fn powf`) defeats the analysis, and
it does so whether or not any intrinsic is involved. See
[ADR 0005](adr/0005-intrinsic-leaf-classification.md) for the measurement
and the alternative.

**`cargo no-alloc -- test` is not usable for realistic code.** Cargo ignores
`[profile.*] panic` for the `test` profile — it always builds tests under
`panic=unwind`, regardless of what the manifest sets — so every `Assert`
terminator rejects under `-- test` no matter how the profile is configured.
`-- build` is the supported mode for meaningful results; `-- test` is useful
only for trivial, assertion-free code and is not a supported path for
checking real workloads.

Cross-crate generic roots are checked at downstream monomorphization sites.
Per-rustc fragments are merged deterministically, and unmatched or non-function
`--root` specifications are reported rather than silently disappearing. A
`#[no_alloc]` marker on a non-generic function in a non-workspace dependency
is a different case — see `--all-crates` above. `async fn` roots (marked or
`--root`) are a selection error, since only the future constructor has MIR.

**Further limitations, not closed by the checker:**

- The guarantee is about the `__rust_alloc`-family entry points (the
  allocator, deallocator, reallocator, and zeroed-allocator codegen
  attributes). A direct method call on a user `#[global_allocator]` static's
  `GlobalAlloc` impl, or on an allocator crate's API, is ordinary code and is
  followed like any other call: it is a violation only if that code reaches
  the entry points.
- The checked build is not the shipped build. It uses the pinned nightly's
  standard library and compiler, sets `--cfg no_alloc_check`, and the marker
  adds `rustc_no_mir_inline` to roots. Code that differs under
  `cfg(no_alloc_check)` or under another toolchain is checked as built here.
- `--root` on an unmarked generic function can still be MIR-inlined into all
  its callers and reported `NotInstantiated`, which counts as success. The
  marker prevents this for marked roots.
- A callee with explicit linkage (`#[linkage = "weak"]` and similar) is
  rejected: the linker may bind the symbol to a different definition than the
  MIR checked. A field projection into a type with an extern-type tail is
  rejected too, because codegen emits a panic call for it that no MIR edge
  names ([ADR 0003](adr/0003-reject-unresolved-edges.md)).

See [`docs/design.md`](docs/design.md) and the [`adr/`](adr) directory for the
design rationale and operational details.

## Development verification

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo llvm-cov --workspace --summary-only -- --skip ui_matrix
cargo bench
cargo audit
```

## Release order

Prepare and publish, with separate explicit authorization, in this order:

1. `no_alloc_report`
2. `no_alloc_check`
3. `no_alloc_analysis`
4. `cargo-no-alloc`

Publishing is not part of repository verification and is never performed
implicitly.

## License

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT).
