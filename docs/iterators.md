# The non-allocating iterator subset

Iterators are the case where "does this reach the allocator?" and "can this
tool prove it?" come apart the furthest. `slice.iter().map(..).sum()`
allocates nothing at runtime, and until the changes in ADRs
[0005](../adr/0005-intrinsic-leaf-classification.md),
[0006](../adr/0006-immediate-abort-checking-mode.md) and
[0007](../adr/0007-shim-and-fn-item-resolution.md) this checker could not
verify a single iterator pattern — not one, in any configuration.

This document is the measured answer: what passes now, what does not, and
why. Everything in the tables comes from
[`examples/iterators`](../examples/iterators), which is 35 `#[no_alloc]`
roots you can re-run yourself.

## The short version

```bash
cargo no-alloc --immediate-abort -- build
```

`--immediate-abort` rebuilds the crate *and* the standard library with
`-Cpanic=immediate-abort`, so every panic lowers to a bare `abort()` with no
handler call. Without it, iterator code does not reject because of anything
it does — it rejects because the panic machinery threaded through
`slice::Iter` has no MIR in the precompiled sysroot.

| Configuration | Roots passing (of 35) |
|---|---|
| default (`panic = "unwind"`) | 0 |
| `[profile.dev] panic = "abort"` | 0 |
| `--immediate-abort` | 33 |

The other two are a real allocation and a real function pointer, both
described below.

## What passes

All of these pass under `--immediate-abort`, on `&[f32]`/`&[u32]` slices:

| Category | Patterns |
|---|---|
| Driving an iterator | `for &x in slice`, `iter.next()`, `iter_mut`, `0..n`, `(0..n).step_by(2)`, `[T; N]::into_iter` |
| Consuming | `sum`, `fold`, `for_each`, `count`, `nth` |
| Searching | `any`, `all`, `find`, `position` |
| Adapters | `map`, `filter`, `enumerate`, `zip`, `chain`, `skip`, `take`, `take_while`, `rev`, `copied`, `peekable` |
| Slice views | `chunks_exact`, `rchunks`, `windows`, `binary_search` |
| Comparison and callbacks | `max`, `max_by(\|a, b\| a.cmp(b))`, `min_by_key`, `flat_map`, `.scan(..).last()` |

Nothing here is a special case in the checker: each one is walked to its
leaves, and the leaves are arithmetic, memory primitives, and `abort`.

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
assume. Nothing about `--immediate-abort` changes that, and nothing should:
this is the tool reporting a real limit on what it can see, not a modelling
gap.

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
`alloc::alloc::__rust_alloc_zeroed`. That is the point of including it: the
mode that makes 33 patterns pass does not make an allocating one pass.

## Why the other configurations get zero

Under the default `panic = "unwind"`, an `Assert` terminator rejects
outright ([ADR 0003](../adr/0003-reject-unresolved-edges.md)) — the
already-documented default-configuration story, and the reason the README
tells you to set `panic = "abort"` before expecting anything to pass.

`panic = "abort"` disposes of the `Assert` problem and gets no further,
because a panic is not only an `Assert`. `slice::Iter::next` calls
`usize::unchecked_sub`; that call's UB-check precondition calls
`core::panicking::panic_nounwind_fmt`, an ordinary `Call` terminator into a
function the precompiled sysroot ships no MIR for. Rejecting it is correct —
the traversal genuinely cannot see what it does. Measured across the 35
roots in this configuration: 34 rejections, every one of them inside the
panic machinery (`panic_nounwind_fmt` 29, `panic_fmt` 3, `panic` 1,
`expect_failed` 1), and the 35th is `collects_into_vec`'s violation. Not one
rejection is at anything that allocates.

`--immediate-abort` removes the machinery instead of excusing it. Panic paths
are then *walked*, and they end at `core::intrinsics::abort`, which
[ADR 0005](../adr/0005-intrinsic-leaf-classification.md)'s intrinsic table
classifies as a leaf that cannot reach the allocator. Note what this does
*not* do: it does not add a panic-path carve-out. In this mode the panic path
is checked like any other.

## Caveats worth stating out loud

- **The verdict is about the build the checker made.** A crate checked with
  `--immediate-abort` and shipped with `panic = "unwind"` has a panic runtime
  in its shipped binary that allocates. What was proven is that the
  *non-panicking* code contains no allocation, and that under an immediate
  abort there is no panic path that allocates either.
- **Verdicts are per-instantiation** ([ADR 0001](../adr/0001-mono-site-analysis.md)).
  `sum::<f32>` passing says nothing about `sum::<MyType>`, whose `Add` impl
  is a different instance.
- **The tables are pinned to `nightly-2026-08-01`.** They are statements
  about how that toolchain's standard library is written, not about the
  `Iterator` API. A std change to how an adapter is specialized can move a
  pattern from one column to the other; that is what
  `tests/ui/iterator_immediate_abort` exists to catch.
- **`--immediate-abort` rebuilds std on every run**, on top of the
  from-scratch rebuild every `cargo no-alloc` run already performs. Budget
  minutes, not seconds.

## Reproducing

```bash
cd examples/iterators
cargo no-alloc --immediate-abort -- build   # 33 pass, 1 reject, 1 violation
cargo no-alloc -- build                     # 0 pass
```

The report each run writes to `target/no-alloc/report.json` is the machine-
readable form of the tables above.
