# Releasing

Publishing is permanent and requires separate explicit authorization. Keep the
version at `0.1.0` for the initial release and recheck crate-name availability
immediately before publishing.

Package and publish in dependency order:

1. `no_alloc_report`
2. `no_alloc_check`
3. `no_alloc_analysis`
4. `cargo-no-alloc`

Before publication, run the repository verification commands, inspect each
`cargo package --list`, install `cargo-no-alloc` into a temporary root, and
verify that both `cargo-no-alloc` and `no-alloc-driver` were installed. This
document does not authorize `cargo publish`.
