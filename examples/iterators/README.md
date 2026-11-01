# iterators

The worked example behind [`docs/iterators.md`](../../docs/iterators.md): 35
`#[no_alloc]` roots covering the iterator patterns that show up in real
non-allocating code, from `for &x in slice` to `sort_unstable`.

## Run

```bash
cd examples/iterators
cargo no-alloc -- build
```

Expect 31 passes, three rejections — `sort_in_place` (which dispatches through
a real `fn` pointer inside the sort), `take_skip` and `zip_dot` (both reach the
not-yet-audited `integer_min` intrinsic) — and one violation:
`collects_into_vec`, which really does allocate, reported with the chain
down to `__rust_alloc_zeroed`. Panic paths are out of scope in this mode
([ADR 0008](../../adr/0008-panic-paths-out-of-scope-by-default.md)).

`cargo no-alloc --strict -- build` checks the panic paths too, by rebuilding
the standard library with `-Cpanic=immediate-abort`, so it takes a couple of
minutes. Measured, it gives the same 31 / 3 / 1
([ADR 0006](../../adr/0006-immediate-abort-checking-mode.md)).

This crate's manifest is ordinary — no `cargo-features`, no
`[profile.dev] panic`: `--strict` supplies the panic strategy for its own
build.
