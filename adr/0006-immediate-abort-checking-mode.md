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

Two guards keep the mode from silently mixing an immediate-abort crate with
a sysroot that was not rebuilt to match, which would make the report claim
a guarantee the build did not earn. Pre-flight, `cargo no-alloc` rejects
`-Cpanic=immediate-abort` arriving through the ambient environment
(`RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS`, in any spelling rustc accepts,
resolved by the *last* `-C panic=...` in the stream since that is the one
rustc uses) unless `--build-std` was also passed — `--immediate-abort`
always implies `--build-std`, but the reverse isn't required: plain
`--build-std` rebuilds the sysroot under the same inherited flags and is
just as coherent as the flag, so gating this guard on `--immediate-abort`
specifically rejected that legitimate combination too. It also rejects a
test-harness target
selection (`--tests`, `--all-targets`, and `-- test` itself) under
`--immediate-abort` before paying for a sysroot rebuild rustc would refuse
anyway (`building tests with panic=abort is not supported without
-Zpanic_abort_tests`). `--test`/`--bench`/`--benches` are deliberately not
in that list — they name or filter to a target that may set
`harness = false` and build fine, and telling those apart would need this
wrapper to parse Cargo's own target metadata, which it does nowhere else.

Neither pre-flight guard can see every route the strategy might take — the
manifest, `config.toml`, or a narrow `--test`/`--bench` selection, none of
which show up in an environment variable — so after the build,
`cargo no-alloc` also checks what the driver actually observed: any
verdict-bearing fragment reporting `ImmediateAbort` without `--build-std` is
rejected, since `--build-std` is what makes an otherwise hand-set strategy
coherent (it rebuilds the sysroot under the same flags the crate itself
used). This check reads the raw fragments directly rather than the merged
report's `panic_strategy` — a target-specific config can select the
strategy for the checked crate while a wrapped host unit (a build script
with a checked root of its own) compiles under something else, and that
disagreement is exactly the case `Report::merge` reports as `None` for
`report.json`'s sake. Using the merged field here would let the mix through
silently; checking every fragment does not.

The rejection lands in the persisted `report.json` itself, not only on
stderr and the process exit code: a mixed sysroot is pushed onto
`selection_errors` (the existing channel for "this report should not be
trusted as-is") before the file is written, so `Report::is_success()`
already says `false` for anyone reading the file directly. Without this, a
trivial root with no reason to care about std's panic strategy would show a
bare `Pass` with an `ImmediateAbort` label and nothing else — indistinguishable
from a coherent run to a consumer that checks the JSON instead of this
process's exit code.

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

### Host units always compile under `unwind`, and that is not a gap

A workspace member's build script or proc-macro is a *host* artifact: Cargo
compiles it to run during the build itself, and — independently of
`[profile.*] panic` and of `RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS`, both of
which apply only to the *target* platform once `--target` is explicit —
always compiles it under the default `unwind` strategy. Verified directly:
a `#[no_alloc]`-marked function placed in `build.rs` reports
`panic_strategy: unwind` for its own fragment whether or not the checked
crate sets `[profile.dev] panic = "abort"`, and whether or not the checker
run passes `--immediate-abort`. `Report::merge` (`no_alloc_report/src/report.rs`)
correctly reports the whole build's `panic_strategy` as `None` in that case,
since the fragments genuinely disagree; that `None` is the honest answer,
not a bug to route around.

It is tempting to read that disagreement as the host root's verdict having
used a weaker, unproven carve-out that the "no carve-out" guarantee above
promised to eliminate. It has not: `unwind` is the *strictest* of the three
panic strategies as far as this traversal is concerned. Its `Assert`
handling is `TerminatorKind::Assert { .. } if tcx.sess.panic_strategy().unwinds() => Unresolved(..)`
— a hard rejection, never the `Edge::None` carve-out `abort` and
`immediate-abort` both give that terminator. So a host root that reaches a
`Pass` under `unwind` has been proven not to reach *any* assert-adjacent
panic machinery at all, which is at least as strong a claim as a `Pass`
under either other strategy, never weaker. The remaining terminals —
`UnwindResume`, `UnwindTerminate`, `Unreachable` — are `Edge::None`
unconditionally in `classify_terminator`, with no strategy check at all
(ADR 0003); that scope exclusion is identical in every mode, including the
target crate's own `--immediate-abort`-checked code, so a host root reaching
one is not weaker than a target root reaching the same terminator under
immediate-abort — it is the same, pre-existing, documented exclusion.

So: a mixed host/target `panic_strategy` in one build never hides an unsound
verdict on either side. Schema version 2 records the complete build
environment on each checked root, so the report now preserves exactly which
strategy and compiler settings established each verdict. The compatibility
`panic_strategy` field at report level is still `None` when fragments
disagree; readers that need a precise answer should use each root's
`environment`.
