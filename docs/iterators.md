# The non-allocating iterator subset

Iterators are the case where "does this reach the allocator?" and "can this
tool prove it?" come apart the furthest. `slice.iter().map(..).sum()`
allocates nothing at runtime, and until the changes in ADRs
[0005](../adr/0005-intrinsic-leaf-classification.md),
[0006](../adr/0006-immediate-abort-checking-mode.md),
[0007](../adr/0007-shim-and-fn-item-resolution.md) and
[0008](../adr/0008-panic-paths-out-of-scope-by-default.md) this checker could
not verify a single iterator pattern in its default configuration.

This document is the measured answer: what passes now, what does not, and
why. Everything in the tables comes from
[`examples/iterators`](../examples/iterators), which is 35 `#[no_alloc]`
roots you can re-run yourself, measured on `nightly-2026-08-01`.

## The short version

```bash
cargo no-alloc -- build            # panic paths out of scope (default)
cargo no-alloc --strict -- build   # panic paths checked too
```

Iterator code reaches the panic machinery everywhere: `slice::Iter::next`
calls `usize::unchecked_sub`, whose UB-check precondition calls
`core::panicking::panic_nounwind_fmt`, and overflow checks in adapters lower
to `Assert` terminators. In the default mode those are panic paths, out of
scope ([ADR 0008](../adr/0008-panic-paths-out-of-scope-by-default.md)): the
walk stops at the panic entry point. Under `--strict` the crate and the
standard library are rebuilt with `-Cpanic=immediate-abort`, so every panic
lowers to a bare `abort()`, and the walk goes all the way down to it.

| Configuration | Roots passing (of 35) |
|---|---|
| default (`panic = "unwind"`) | 33 |
| default, `[profile.dev] panic = "abort"` | 33 |
| `--strict` | 33 |

The other two are a real allocation and a real function pointer, both
described below, and they are the same two in every configuration. Before
ADR 0008 the first two rows were 0: every root rejected inside the panic
machinery, at a function the precompiled sysroot ships no MIR for.

The rows mean different things. A default-mode `Pass` says no allocator is
reachable except through a panic; a `--strict` `Pass` also says the panic
paths (compiled to `abort`) cannot allocate. `report.json` records which, as
`panic_paths` on each root's `environment`.

## What passes

All of these pass in every configuration above, on `&[f32]`/`&[u32]` slices:

| Category | Patterns |
|---|---|
| Driving an iterator | `for &x in slice`, `iter.next()`, `iter_mut`, `0..n`, `(0..n).step_by(2)`, `[T; N]::into_iter` |
| Consuming | `sum`, `fold`, `for_each`, `count`, `nth` |
| Searching | `any`, `all`, `find`, `position` |
| Adapters | `map`, `filter`, `enumerate`, `zip`, `chain`, `skip`, `take`, `take_while`, `rev`, `copied`, `peekable` |
| Slice views | `chunks_exact`, `rchunks`, `windows`, `binary_search` |
| Comparison and callbacks | `max`, `max_by(\|a, b\| a.cmp(b))`, `min_by_key`, `flat_map`, `.scan(..).last()` |

Nothing here is a special case in the checker: each one is walked to its
leaves, and the leaves are arithmetic, memory primitives, and either a panic
entry point (default) or `abort` (`--strict`).

## What is still rejected

One root rejects, and it earns it: `slice.sort_unstable()`.

```
sorted
  core::slice::<impl [T]>::sort_unstable
    core::slice::sort::unstable::sort
      core::slice::sort::unstable::ipnsort
        core::slice::sort::unstable::quicksort::quicksort
          core::slice::sort::unstable::quicksort::partition
            callee is a function pointer, not a statically resolvable body
```

`partition` selects its implementation through an actual `fn` pointer, so
there is no single body for the traversal to walk to, and
[ADR 0003](../adr/0003-reject-unresolved-edges.md) says reject rather than
assume. Neither mode changes that, and nothing should: this is the tool
reporting a real limit on what it can see, not a modelling gap.

Comparison-driven adapters used to be in this section for a much worse
reason. `max`, `min_by_key`, `flat_map` and `.scan(..).last()` reach their
callbacks as function items behind a `FnMut::call_mut` shim, and the
traversal rejected the shim without looking inside it —
[ADR 0007](../adr/0007-shim-and-fn-item-resolution.md) is that fix. If you
are reading an older report, that is what those `FnMut::call_mut` chains
were.

## The control case

`collects_into_vec` — `iter.copied().collect::<Vec<_>>()` — is reported as a
violation in *every* configuration, with the chain down to
`alloc::alloc::__rust_alloc_zeroed`. That is the point of including it:
neither leaving panic paths out of scope nor rebuilding std makes an
allocating pattern pass.

## Caveats worth stating out loud

- **The default verdict excludes panic paths.** With `std`, a panic formats
  and prints, which allocates; the default mode does not look. If that
  matters, use `--strict`, whose verdict covers the panic paths as compiled
  to `abort`.
- **A `--strict` verdict is about the build the checker made.** A crate
  checked with `--strict` and shipped with `panic = "unwind"` has a panic
  runtime in its shipped binary that allocates. What was proven is that the
  *non-panicking* code contains no allocation, and that under an immediate
  abort there is no panic path that allocates either.
- **Verdicts are per-instantiation** ([ADR 0001](../adr/0001-mono-site-analysis.md)).
  `sum::<f32>` passing says nothing about `sum::<MyType>`, whose `Add` impl
  is a different instance.
- **The tables are pinned to `nightly-2026-08-01`.** They are statements
  about how that toolchain's standard library is written, not about the
  `Iterator` API. A std change to how an adapter is specialized can move a
  pattern from one column to the other; that is what
  `tests/ui/strict_iterator` and `tests/ui/panic_out_of_scope` exist to
  catch.
- **`--strict` rebuilds std on every run**, on top of the from-scratch
  rebuild every `cargo no-alloc` run already performs. Budget minutes, not
  seconds. The default mode does not.

## Reproducing

```bash
cd examples/iterators
cargo no-alloc -- build            # 33 pass, 1 reject, 1 violation
cargo no-alloc --strict -- build   # 33 pass, 1 reject, 1 violation
```

The report each run writes to `target/no-alloc/report.json` is the machine-
readable form of the tables above.
