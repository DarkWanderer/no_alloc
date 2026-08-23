# ADR 0001: Mono-site, not definition-site, analysis

## Status

Accepted.

## Context

A function can be analyzed at its definition (once, generically) or at each
monomorphized instantiation (once per concrete type substitution). Rust's
generics make these genuinely different questions. A generic function like:

```rust
fn store<T>(x: T) { HEAP.push(x); }
```

does not allocate for `T = [f32; 4]` if `HEAP` were, say, a fixed-capacity
stack buffer generic over element size — but the point generalizes: whether a
generic function allocates can depend on which concrete type parameters it is
instantiated with, because trait dispatch, drop glue, and specialization all
resolve differently per instantiation.

## Decision

Analysis runs on the monomorphized instance graph: `Instance<'tcx>` nodes
from `tcx.collect_and_partition_mono_items(())`, not `DefId` nodes. A root
function's verdict is reported per-instantiation. Two instantiations of the
same generic root can have different verdicts (this is required test case
"Generic fn, one instantiation allocating, one not").

## Consequences

- Generic resolution comes free: by the time we're looking at an `Instance`,
  all type parameters are concrete and `Instance::try_resolve` can follow
  trait method calls to their concrete implementation without us writing any
  trait-resolution logic ourselves.
- The guarantee is per-instantiation, not a modular signature contract. A
  generic function has no single "does it allocate" answer independent of
  how it's used — the tool does not pretend otherwise.
- A generic root with no instantiation in the crate under analysis has
  nothing to check; this is reported as `NotInstantiated` (info), not a
  failure, since there is no MIR to walk.
- The analysis can only run where codegen happens (`cargo build`/`cargo
  test`, not `cargo check`), because the mono item graph is a codegen
  artifact. See the Invocation section of `docs/design.md`.

## Alternatives considered

**Definition-site (modular) analysis**, treating each generic function once
with an abstract signature contract (e.g. "allocates iff `T::method`
allocates"). Rejected: it either requires trait bounds strong enough to make
the contract sound (which most real code doesn't have and which the tool
cannot infer), or it degrades to the same "reject on anything
non-concrete" conservatism as mono-site analysis while being harder to
implement and explain. Mono-site gets the same guarantee for free from
`rustc`'s own monomorphization.
