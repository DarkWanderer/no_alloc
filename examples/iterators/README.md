# iterators

The worked example behind [`docs/iterators.md`](../../docs/iterators.md): 35
`#[no_alloc]` roots covering the iterator patterns that show up in real
non-allocating code, from `for &x in slice` to `sort_unstable`.

## Run

```bash
cd examples/iterators
cargo no-alloc --immediate-abort -- build
```

Expect 33 passes, one rejection — `sort_in_place`, which dispatches through
a real `fn` pointer inside the sort — and one violation:
`collects_into_vec`, which really does allocate, reported with the chain
down to `__rust_alloc_zeroed`.

`--immediate-abort` rebuilds the standard library, so the first run takes a
couple of minutes. It is also what makes any of this checkable: run the same
command without the flag and all 33 passes become rejections inside the
panic machinery, which is what
[ADR 0006](../../adr/0006-immediate-abort-checking-mode.md) is about.

Note that this crate's manifest is entirely ordinary — no
`cargo-features`, no `[profile.dev] panic`. The checker supplies the panic
strategy for its own build, so the crate still builds on stable.
