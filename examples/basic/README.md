# basic

A minimal crate demonstrating `no_alloc` on two functions:

- `safe_sum` — destructures a fixed-size array (`let [a, b, c] = *buf;`) and
  sums the elements. Passes.
- `unsafe_alloc` — calls `Box::new`. Flagged as a violation, with the full
  call chain down to the allocator.

## Run

```bash
cd examples/basic
cargo no-alloc -- build
```

Expect one `error: no_alloc: this function may reach the global allocator`
for `unsafe_alloc`, and no error for `safe_sum`.

## Note on iterators

Writing `safe_sum` as `for &x in buf { ... }` instead of the array destructure
above makes it *rejected* rather than pass: the slice iterator's `next()`
goes through an `unchecked_sub` precondition check that calls
`core::panicking::panic_nounwind_fmt`, and the precompiled sysroot ships no
MIR for it. The tool follows "reject, don't assume" for any unresolved call
edge (see [ADR 0003](../../adr/0003-reject-unresolved-edges.md)), so this is
flagged rather than approved. Try swapping the loop to see it for yourself.

Then run it again as `cargo no-alloc --immediate-abort -- build`: with the
standard library rebuilt so panics lower to a bare `abort()`, the same loop
passes, because the traversal can now walk that path to its end. The full
picture is in [`docs/iterators.md`](../../docs/iterators.md), with
[`examples/iterators`](../iterators) as the worked example.
