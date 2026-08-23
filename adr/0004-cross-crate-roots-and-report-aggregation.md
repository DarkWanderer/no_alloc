# ADR 0004: Discover cross-crate roots at mono sites and aggregate reports

## Status

Accepted.

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
cannot lose reports through last-writer-wins races, and requested roots cannot
disappear silently.
