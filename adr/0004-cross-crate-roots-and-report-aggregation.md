# ADR 0004: Discover cross-crate roots at mono sites and aggregate reports

## Status

Accepted, amended 2026-10-30.

## Context

A generic root defined in a dependency has no concrete body to analyze in that
dependency. Its allocation behavior depends on downstream substitutions.
Parallel rustc processes also cannot safely overwrite one shared report.

## Decision

Each analyzed crate scans its complete monomorphized item set. A function
instance is selected when its local or foreign definition carries the root
attribute, or its canonical path matches a requested root. Verdicts remain
per-instance.

Each rustc process writes an atomic, process-unique fragment. The Cargo wrapper
holds a workspace lock, merges all fragments deterministically, validates root
specification matches across the build, and atomically publishes one final
report. A concrete instance supersedes a definition-only `NotInstantiated`
entry for the same canonical root.

## Consequences

No sidecar root index is needed. Cross-crate metadata supplies the marker and
the downstream mono graph supplies the executable instance. Multi-crate builds
cannot lose reports through last-writer-wins races, and a requested `--root`
specification cannot disappear silently — an unmatched one is reported as a
selection error. This does not extend to `#[no_alloc]` markers themselves: in
the default (non-`--all-crates`) invocation mode, a marker in a non-workspace
dependency is never instrumented and never appears in the report at all,
silently (see `README.md`'s `--all-crates` documentation).

### Upstream-shared generic instantiations (amended 2026-10-30)

Under `-Zshare-generics` (default at opt-level 0, and always for
`#[inline(never)]`), a generic instantiation an upstream crate already emitted
is linked to rather than codegened locally, so it is absent from the local
`collect_and_partition_mono_items` set. A marked generic root instantiated that
way was silently never discovered in the default mode. `discover_upstream_shared`
now walks from every local mono item through resolved callees (via the
traversal's own `resolved_callees`, so both agree on what a call resolves to),
entering only callees `should_codegen_locally` rejects, and adopts any marked
*generic* root it meets. The restriction to generic roots keeps the rule above
intact: a marked non-generic function in a non-workspace dependency still
requires `--all-crates`. Pinned by `tests/ui/cross_crate_shared_generic`.

The root marker also emits `rustc_no_mir_inline` (ADR 0002), because a marked
generic root MIR-inlined into every caller has no mono item and would be
reported `NotInstantiated`, which counts as success. `--root`-selected unmarked
generic functions have no marker and remain exposed to this.
