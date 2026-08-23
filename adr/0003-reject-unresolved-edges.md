# ADR 0003: Reject unresolved call edges, never assume them benign

## Status

Accepted.

## Context

A DFS over the mono-instance graph will encounter call edges it cannot
statically resolve to a single callee body: `dyn Trait` dispatch, function
pointers (including closures called through a pointer), `extern` calls with
no body in this crate, and inline assembly (which can call anything, or
nothing, opaquely). Each of these could, in principle, be handled with a
bespoke heuristic (e.g. "trust `extern "C"` calls to a short allowlist",
"assume `dyn` calls are safe if the trait has no default-alloc method").

## Decision

Every one of these is the same case: **no statically available callee body**.
All of them are rejected — reported as `Rejected`, distinct from both `Pass`
and `Violation` — never assumed safe, and never silently skipped.

Rejection is **not path-sensitive**. A `dyn` call inside a branch that can
provably never execute (e.g. `if false { obj.method() }`) is still rejected.
The DFS does not attempt to prove branches dead; doing so would reintroduce
exactly the kind of "trust me, this path doesn't run" reasoning this ADR
rejects for callees.

## Consequences

- One principle instead of four special cases. Adding a new opaque
  terminator kind in the future (should MIR grow one) defaults to reject
  without a design discussion.
- False positives are expected and are the user's problem to route around
  (e.g. by restructuring to avoid `dyn` dispatch on a hot path, or by
  narrowing what's inside a `#[no_alloc]` root). This is explicitly
  preferred over any false negative.
- The required test matrix includes a `dyn` call in a dead branch
  specifically to pin non-path-sensitivity as intended behavior, not a gap.
- `Drop` glue is walked explicitly (`Instance::resolve_drop_glue`) rather
  than ignored, because an implicit drop that reaches the allocator is just
  as real a violation as an explicit call — omitting it would be a silent
  soundness hole, not a conservative rejection.

## Alternatives considered

**Allowlisting specific `extern "C"` functions known not to allocate**:
rejected for the leaf binary/test crate use case this tool targets — an
allowlist is a second source of truth that can drift from reality, and the
brief's guarantee is "the callee body is statically available", not "a human
asserted this is fine".

**Best-effort devirtualization for `dyn` calls with a single implementor**:
deferred, not rejected outright as a future direction, but out of scope for
this design. It would need to be sound against adding a second implementor
anywhere in the crate graph without re-running the check, which is a much
harder invalidation problem than this tool takes on.
