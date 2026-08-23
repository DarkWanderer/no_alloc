# basic

A minimal crate demonstrating `no_alloc` on two functions:

- `safe_sum` — an index-based loop over a slice. Passes.
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

Writing `safe_sum` as `for &x in buf { ... }` instead of the index-based loop
above makes it *rejected* rather than pass: in debug builds, the slice
iterator's `next()` goes through an `unchecked_sub` precondition check that
has no statically available MIR body. The tool follows "reject, don't
assume" for any unresolved call edge (see
[ADR 0003](../../adr/0003-reject-unresolved-edges.md)), so this is flagged
rather than approved. Try swapping the loop to see it for yourself.
