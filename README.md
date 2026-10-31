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

Add this lint configuration to consumers:

```toml
[lints.rust]
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(no_alloc_check)'] }
```

Run the checker with Cargo arguments after `--`:

```bash
cargo no-alloc -- build
```

See [`docs/usage.md`](docs/usage.md) for the full command-line and
configuration reference.

## Guarantee and limitations

The checker is conservative: unresolved call edges (function pointers,
virtual dispatch, foreign calls without MIR) are rejected, never assumed
safe. Allocation reachable only through a panic path is excluded unless you
pass `--immediate-abort`, which rebuilds the standard library so panics lower
to a bare abort. The boundary is MIR, not machine code, so symbols the
backend synthesizes are assumed to be the platform's. `cargo no-alloc -- test`
is not usable for realistic code. See [`docs/guarantees.md`](docs/guarantees.md)
for the full statement.

## Documentation

- [Usage](docs/usage.md): command line, configuration, and report format
- [Guarantee and limitations](docs/guarantees.md)
- [Iterators](docs/iterators.md): what passes under `--immediate-abort`
- [Design](docs/design.md)
- [Development](docs/development.md)
- [Releasing](docs/releasing.md)
- [Architecture decision records](adr/)

## License

Licensed under either [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT).
