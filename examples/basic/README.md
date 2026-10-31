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
above also passes: the slice iterator's `next()` goes through an
`unchecked_sub` precondition check that calls
`core::panicking::panic_nounwind_fmt`, which is a panic entry point, and
panic paths are out of scope by default
([ADR 0008](../../adr/0008-panic-paths-out-of-scope-by-default.md)).

Run it as `cargo no-alloc --strict -- build` to check panic paths as well:
the standard library is rebuilt so panics lower to a bare `abort()`, and the
traversal walks that path to its end instead of stopping at it. The full
picture is in [`docs/iterators.md`](../../docs/iterators.md), with
[`examples/iterators`](../iterators) as the worked example.
