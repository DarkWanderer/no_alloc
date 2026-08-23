# no_alloc

`no_alloc` statically checks that selected Rust function instances cannot
reach the global allocator. It is intentionally conservative: unresolved
calls are rejected instead of assumed safe.

The checker is currently Linux-only and tied to `nightly-2026-08-01`.

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
cargo no-alloc [--all-crates] [--build-std] [--warn-only]
               [--root PATH]... -- [build|test] [CARGO_ARGS...]
```

`build` is the default. `check` is rejected because it does not produce the
monomorphized graph. The checker owns its target and target directory, so
user-supplied `--target` and `--target-dir` are rejected. `--root` selects an
unannotated function by canonical path across the complete build.

By default only workspace members are instrumented (`RUSTC_WORKSPACE_WRAPPER`).
A `#[no_alloc]` marker on a non-generic function in a registry or path
dependency outside the workspace is therefore silently never analyzed — the
function never appears in the report at all, which is different from an
unmatched `--root` (reported as a selection error). Pass `--all-crates` to
instrument every crate in the build (`RUSTC_WRAPPER`) instead.

The existing `NO_ALLOC_ROOTS`, `NO_ALLOC_WARN_ONLY`, and `NO_ALLOC_LOG`
environment interfaces remain supported. The final deterministic report is
written to `target/no-alloc/report.json`.

See [`examples/basic`](examples/basic) for a runnable example.

## Guarantee and limitations

Analysis starts from each selected monomorphized `Instance`, follows resolved
calls, tail calls, and drop glue, and detects allocator, deallocator,
reallocator, and zeroed-allocator terminals. Function pointers, virtual
dispatch, foreign calls without MIR, and inline assembly are rejected. A
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
guarantee, not because that allocation has been ruled out. The one way to
remove the call entirely is `panic_immediate_abort`, a `-Zbuild-std` std
feature that compiles every panic to a bare `abort()` with no handler call.
Under `panic=unwind`, `Assert` is rejected outright instead, on the usual
reject-don't-assume grounds — see [ADR 0003](adr/0003-reject-unresolved-edges.md)
for the full terminator classification and rationale.

In practice this means most indexing and arithmetic in debug builds — which
lower to an `Assert` — reject under `panic=unwind` (Rust's default); set
`panic = "abort"` in `[profile.*]` for realistic code to pass at all. See
[`examples/basic`](examples/basic) for the equivalent tradeoff with iterators.

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

1. `no_alloc_check`
2. `no_alloc_report`
3. `no_alloc_analysis`
4. `cargo-no-alloc`

Publishing is not part of repository verification and is never performed
implicitly.

## License

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT).
