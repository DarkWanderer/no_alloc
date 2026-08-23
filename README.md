# no_alloc

A tool that **proves** a designated set of Rust functions cannot reach the
global allocator on any execution path.

The motivating use case is latency-sensitive code (audio, real-time control,
kernel-adjacent) where a single `malloc` lock acquisition is a defect. The
guarantee is the product: any call edge whose callee body is not statically
available is **rejected**, never assumed benign. False positives are the
user's problem; false negatives are bugs in this tool.

See [`docs/design.md`](docs/design.md) for the full design and the `adr/`
directory for the load-bearing decisions.

## How it works, in one paragraph

`cargo-no-alloc` runs `cargo build`/`cargo test` with an out-of-tree
`rustc_driver` binary (`no-alloc-driver`) substituted in as the compiler. The
driver walks the **monomorphized instance graph** — not function definitions —
starting from functions marked `#[no_alloc::no_alloc]`, follows every `Call`,
`TailCall`, and `Drop` edge it can statically resolve, and checks whether any
resolved callee is an allocator/deallocator terminal. Any edge it cannot
resolve statically (`dyn` dispatch, function pointers, inline asm, or a
MIR-less callee) is a rejection, not an assumption of safety.

## Usage

```rust
#[no_alloc::no_alloc]
fn process_audio_block(buf: &mut [f32]) {
    // ...
}
```

```bash
cargo no-alloc -- build
```

### Required lint configuration

The marker expands to a `cfg_attr` that is inert on normal builds, but the
`no_alloc_check` cfg it checks for is still unknown to `rustc` outside the
checker driver. Add this to your crate to silence `unexpected_cfgs`:

```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(no_alloc_check)'] }
```

This is the one piece of friction the design imposes on normal builds; see
[ADR 0002](adr/0002-cfg-gated-tool-attribute-marker.md) for why.

## Toolchain

Pinned via `rust-toolchain.toml` to `nightly-2026-08-01`, with the
`rustc-dev`, `rust-src`, and `llvm-tools-preview` components the driver needs
to link against `librustc_driver`. First run:

```bash
rustup toolchain install nightly-2026-08-01 -c rustc-dev -c rust-src -c llvm-tools-preview
```

## Verification

```bash
cargo build --workspace
cargo test --workspace
cargo llvm-cov --workspace --summary-only -- --skip ui_matrix
cargo bench
```

The `--skip ui_matrix` matters: `tests/ui.rs` exercises the analysis by
spawning the built `cargo-no-alloc` binary as a subprocess, which
`cargo llvm-cov` can't instrument — running it under coverage produces a
spurious failure with no coverage benefit. See `docs/design.md` for why.

## Status

M0–M5 complete: driver, analysis, diagnostics, the full required 13-fixture
regression suite (`tests/ui/`), macro unit tests, benchmarks, and coverage
are all built and passing. See `docs/design.md` for what each milestone
verified against the real toolchain (not assumed from docs for a different
nightly) and the couple of deliberately-scoped gaps (sidecar root-index
read/union across workspace crates; `-Zbuild-std`, measured off).
