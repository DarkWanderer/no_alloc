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
canonical path across the complete build.

By default only workspace members are instrumented (`RUSTC_WORKSPACE_WRAPPER`).
A `#[no_alloc]` marker on a non-generic function in a registry or path
dependency outside the workspace is therefore silently never analyzed — the
function never appears in the report at all, which is different from an
unmatched `--root` (reported as a selection error). Pass `--all-crates` to
instrument every crate in the build (`RUSTC_WRAPPER`) instead.

The existing `NO_ALLOC_ROOTS`, `NO_ALLOC_WARN_ONLY`, and `NO_ALLOC_LOG`
environment interfaces remain supported. The final deterministic report is
written to `target/no-alloc/report.json`.

Every invocation runs `cargo clean` on the checker's target first, so
**every `cargo no-alloc` run is a from-scratch rebuild**, never an
incremental one. This is deliberate — it guarantees that a checker
configuration change can never be hidden by Cargo's build cache (see
[`docs/design.md`](docs/design.md)) — but it means every run pays full
compile time, not just the cost of what changed.

`--immediate-abort` rebuilds the crate and the standard library with
`-Cpanic=immediate-abort` (implying `--build-std`), which is what makes
iterator-shaped code checkable at all — see
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

**The guarantee covers only the non-panicking execution paths of a root.**
`Unreachable`, `UnwindResume`, and `UnwindTerminate` have no MIR callee to
follow, so the traversal treats them as terminal control flow rather than
calls — but each one lowers to a real call during codegen (into the panic
runtime's `panic_fmt`, a foreign `_Unwind_Resume`, or a nounwind panic-handler
call) that the traversal never walks. An `Assert` terminator is allowed only
under a non-unwinding panic strategy (`panic=abort`), and that is *not*
because the panic runtime is unreachable there: a failing assertion under
plain `panic=abort` still calls `panic_fmt` and the `#[panic_handler]`
(which, with `std`, formats and prints — i.e. allocates) before the process
aborts. It is allowed because allocation reachable only through a panic, an
unwind-terminate, or an unwind-resume path is out of scope for this
guarantee, not because that allocation has been ruled out. Under
`panic=unwind`, `Assert` is rejected outright instead, on the usual
reject-don't-assume grounds — see [ADR 0003](adr/0003-reject-unresolved-edges.md)
for the full terminator classification and rationale.

**`--immediate-abort` is the exception, and the stronger guarantee.** It
compiles the crate and the standard library with `-Cpanic=immediate-abort`,
where every panic — `Assert` included — lowers to a bare `abort()` with no
handler call. The traversal then walks panic paths as ordinary edges and
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
`#[no_alloc]` marker in a non-workspace dependency is a different case — see
`--all-crates` above.

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
