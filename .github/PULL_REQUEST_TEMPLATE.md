## What and why

<!-- What does this change, and why? -->

## Checklist

- [ ] Ran the verification commands in [CONTRIBUTING.md](../CONTRIBUTING.md#build-and-test) (`cargo fmt`, `cargo clippy`, `cargo test --workspace`, `cargo llvm-cov`, `cargo bench`, `cargo audit`)
- [ ] If this changes an edge-table rule or the leaf set: added a new fixture under `tests/ui/<case>/`, not just an assertion bolted onto an existing case (see [CONTRIBUTING.md](../CONTRIBUTING.md#regression-tests))
- [ ] If this touches `crates/no_alloc_analysis`: preserves "reject, don't assume" for unresolved call edges ([ADR 0003](../adr/0003-reject-unresolved-edges.md); see [CONTRIBUTING.md](../CONTRIBUTING.md#soundness))
