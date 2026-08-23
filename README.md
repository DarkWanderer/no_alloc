# no_alloc

`no_alloc` statically checks that selected Rust function instances cannot
reach the global allocator. It is intentionally conservative: unresolved
calls are rejected instead of assumed safe.

The checker is currently Linux-only and tied to `nightly-2026-08-01`.

## Install and use

Install the marker macro first, then the Cargo command:

```bash
cargo install no_alloc_check --version 0.1.0
cargo install cargo-no-alloc --version 0.1.0
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

The existing `NO_ALLOC_ROOTS`, `NO_ALLOC_WARN_ONLY`, and `NO_ALLOC_LOG`
environment interfaces remain supported. The final deterministic report is
written to `target/no-alloc/report.json`.

See [`examples/basic`](examples/basic) for a runnable example.

## Guarantee and limitations

Analysis starts from each selected monomorphized `Instance`, follows resolved
calls, tail calls, and drop glue, and detects allocator, deallocator,
reallocator, and zeroed-allocator terminals. Function pointers, virtual
dispatch, foreign calls without MIR, and inline assembly are rejected.

`Unreachable`, `UnwindResume`, and `UnwindTerminate` are terminal control flow,
not calls, and are treated as safe. Synthesized terminate-on-unwind actions are
also safe. An `Assert` terminator is safe only when rustc definitively reports a
non-unwinding panic strategy (`panic=abort` or immediate abort); with
`panic=unwind`, it is rejected because the panic handler is outside the modeled
call graph.

Cross-crate generic roots are checked at downstream monomorphization sites.
Per-rustc fragments are merged deterministically, and unmatched or non-function
root specifications are reported rather than silently disappearing.

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
