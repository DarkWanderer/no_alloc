# Releasing

Publish the crates in dependency order:

1. `no_alloc_report`
2. `no_alloc_check`
3. `no_alloc_analysis`
4. `cargo-no-alloc`

## Before publishing

- Run the verification commands in [CONTRIBUTING.md](../CONTRIBUTING.md).
- Inspect `cargo package --list` for each crate.
- Install `cargo-no-alloc` into a temporary root and confirm that both the
  `cargo-no-alloc` and `no-alloc-driver` binaries are installed.
