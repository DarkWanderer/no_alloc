# ADR 0006: `--immediate-abort` checks panic paths instead of excluding them

## Status

Accepted 2026-08-25.

## Context

ADR 0003 places panic paths outside the guarantee: an `Assert` terminator
under a non-unwinding panic strategy is not traversed, because it has no MIR
callee, and the codegen-time call it lowers to (`panic_fmt`, the
`#[panic_handler]`, which with `std` formats and prints — i.e. allocates) is
never walked. That exclusion is stated in the README, and it is the honest
description of what plain `panic = "abort"` gives.

It is also not enough to check any real code. A panic path is not only an
`Assert` terminator: the standard library reaches the same machinery through
ordinary calls, and those *are* edges the traversal must follow.
`slice::Iter::next` calls `usize::unchecked_sub`, whose UB-check precondition
calls `core::panicking::panic_nounwind_fmt`; the precompiled sysroot ships no
MIR for that function, so the edge is unresolved and rejects. Measured
against `docs/iterators.md`'s twenty patterns under `panic = "abort"`:
**zero** pass, every one of them rejecting inside the panic machinery
(`panic_nounwind_fmt`, `panic_fmt`, `panic`, `expect_failed`) and not one of
them at anything that allocates.

`-Zbuild-std` alone does not fix it either. It makes the sysroot's MIR
available, so the traversal can walk the panic path for real — but with the
panic runtime still in place, what it walks into is the `#[panic_handler]`,
which with `std` formats and prints. Reporting that path is truthful, and
useless: the panic path stops being a carve-out and starts being a finding.

The README's suggested escape hatch, `-Zbuild-std-features=panic_immediate_abort`,
no longer exists: on the pinned nightly it is a real panic strategy
(`-Cpanic=immediate-abort`), and asking for it as a std feature is a
`compile_error!`.

## Decision

`cargo no-alloc --immediate-abort` builds the crate *and the standard
library* with `-Cpanic=immediate-abort`, by adding `-Zunstable-options
-Cpanic=immediate-abort` to the flags it already injects and implying
`--build-std` (the precompiled sysroot was built with a different strategy;
mixing them is what the flag exists to avoid).

Under that strategy every panic — `Assert`, `panic_fmt`, `expect_failed`,
`panic_nounwind_fmt` — compiles to a bare `abort()` with no handler call.
Combined with ADR 0005's intrinsic table, the traversal now *walks* the panic
path to its end and finds `core::intrinsics::abort` there. Nothing is
excluded from the guarantee in this mode: the panic path is checked, and it
passes because it genuinely cannot allocate.

The panic strategy is supplied by the checker rather than by the checked
crate's manifest on purpose. The manifest spelling
(`[profile.dev] panic = "immediate-abort"`) requires
`cargo-features = ["panic-immediate-abort"]`, which makes the whole manifest
nightly-only and would break the crate's ordinary stable build — exactly the
footprint ADR 0002 exists to avoid. `tests/ui/iterator_immediate_abort` is
therefore a fixture with an entirely ordinary manifest.

## Consequences

`--immediate-abort` is the mode in which realistic code can be checked, and
`docs/iterators.md` documents what passes in it. It costs a full sysroot
rebuild on every run (on top of the from-scratch rebuild every
`cargo no-alloc` run already performs).

The two modes now say genuinely different things, and the difference is
worth stating when a result is reported:

| Mode | Panic paths |
|---|---|
| default (`panic = "unwind"`) | `Assert` rejects; nothing checkable |
| `panic = "abort"` | `Assert` out of scope, not proven (ADR 0003); explicit panic calls reject |
| `--immediate-abort` | traversed and checked; they end in `abort` |

Because those three mean different things, `report.json` records the panic
strategy the build actually used (`no_alloc_report::PanicStrategy`, taken
from `tcx.sess.panic_strategy()` per fragment rather than from the flag the
user typed, since `RUSTFLAGS` can set it too). A `Pass` read back later is
otherwise uninterpretable: nothing in it would say whether panic paths were
excluded or checked.

The guarantee under `--immediate-abort` is the strongest of the three and is
the only one with no panic-path carve-out. It says nothing about the
program's behaviour under its *own* panic strategy: a crate that ships with
`panic = "unwind"` and is checked with `--immediate-abort` has been checked
as compiled for that run, and its shipped binary still has a panic runtime
that allocates.
