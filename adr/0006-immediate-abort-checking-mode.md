# ADR 0006: `--immediate-abort` checks panic paths instead of excluding them

## Status

Accepted 2026-08-25, amended 2026-10-24 and 2026-10-30. Superseded in part
2026-10-31 by ADR 0008: the flag is now `--strict` (`--immediate-abort` is
removed), it also tells the analysis to walk panic entry points rather than
stop at them, and it is no longer the only mode in which realistic code
passes, since the default now leaves panic paths out of scope. Everything
below about the strategy, the sysroot rebuild and the guards is unchanged and
reads with `--strict` in place of `--immediate-abort`; the "Consequences"
table describes the `--strict` analysis, not the current default.

## Context

ADR 0003 places panic paths outside the guarantee: an `Assert` terminator
under a non-unwinding panic strategy is not traversed, because it has no MIR
callee, and the codegen-time call it lowers to (`panic_fmt`, the
`#[panic_handler]`, which with `std` formats and prints — i.e. allocates) is
never walked. That exclusion is stated in `docs/guarantees.md`, and it is the honest
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

Under that strategy every panic — `panic_fmt`, `expect_failed`,
`panic_nounwind_fmt`, and the panic lang items a failing `Assert` is lowered
to call — compiles to a bare `abort()` with no handler call. Combined with
ADR 0005's intrinsic table, the traversal now *walks* the panic path to its
end and finds `core::intrinsics::abort` there. For `Assert` that means
resolving the lang item codegen calls and walking it (amended 2026-10-21:
it was previously skipped as under `panic=abort`, so this mode's claim
rested on an assumption rather than a check; `tests/ui/strict_assert`
pins it). Nothing is
excluded from the guarantee in this mode: the panic path is checked, and it
passes because it genuinely cannot allocate.

The panic strategy is supplied by the checker rather than by the checked
crate's manifest on purpose. The manifest spelling
(`[profile.dev] panic = "immediate-abort"`) requires
`cargo-features = ["panic-immediate-abort"]`, which makes the whole manifest
nightly-only and would break the crate's ordinary stable build — exactly the
footprint ADR 0002 exists to avoid. `tests/ui/strict_iterator` is
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
manifest, a `[profile.*] panic` key in `config.toml`, `--profile`, or a narrow
`--test`/`--bench` selection, none of which show up in the rustflags — so
after the build,
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

### Amended 2026-10-24: config rustflags

The first version of this ADR said the strategy "can arrive through the
manifest, `config.toml`, ..." and left it to the post-build check. That was
too coarse for `config.toml` *rustflags*, and the wrapper had a worse problem
with them: it always exported `RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS`, which
Cargo prefers over `build.rustflags` and `target.<triple>.rustflags`, so a
project's configured flags (`--cfg heap`, say) were silently dropped and the
checked build differed from the shipped one.

When the caller sets neither variable, `cargo no-alloc` now reads the
effective `build.rustflags` / `target.<host>.rustflags` with
`cargo config get` and exports them ahead of its own flags, using Cargo's
precedence (a non-empty target list shadows `build`; an empty one does not, and they
never merge). Those flags
are also what the pre-flight guard above inspects, so
`-Cpanic=immediate-abort` in config rustflags is now caught before the build
rather than only after. When the caller does set a variable, Cargo would have
ignored config, so nothing is imported. `target.'cfg(...)'.rustflags` is
refused (recorded in `selection_errors`, no build run) rather than evaluated.
A `build.rustc-wrapper` / `build.rustc-workspace-wrapper` in config is
reported as a warning and neutralised for the run: the build blanks the wrapper
variable it does not use and sets the other to the driver, and an empty
variable disables only its own config key. The `RUSTC_WRAPPER` and
`RUSTC_WORKSPACE_WRAPPER` environment variables are still refused.

Amended 2026-10-30: the same treatment covers the compiler and target. The
driver links its own rustc and ignores the binary Cargo passes it, so a
substitute `rustc` would shape the shipped build and not the checked one.
`RUSTC`, `CARGO_BUILD_RUSTC` and a configured `build.rustc` are refused (a
first version warned about and overrode `build.rustc`, which let a script
adding `--cfg heap` change `cargo build` but not the checked build); the build
is handed the `PATH`-resolved, validated `rustc` as `RUSTC`, and the driver
fails unless its argv[1] is that path (`NO_ALLOC_EXPECTED_RUSTC`). A configured
`build.target`, and a workspace member's `default-target` / `forced-target`
(read from the manifest, since `cargo metadata` does not expose them), are
refused, since the checker always builds `--target <host>`. Cargo's `[env]` table fills any
variable a spawned process leaves unset, so `[env]` keys starting `NO_ALLOC_`
are refused and `NO_ALLOC_WARN_ONLY` is now always set explicitly (`1` or `0`)
rather than removed. Pinned by `config_build_rustc`, `config_build_target` and
`config_env_no_alloc`.
A strategy chosen by a manifest or config *profile* is still seen only by the
post-build check.

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
— a hard rejection, never the `Edge::None` carve-out plain `abort` gives that
terminator (under `immediate-abort` it is traversed into the panic lang item,
per the 2026-10-21 amendment to ADR 0003). So a host root that reaches a
`Pass` under `unwind` has been proven not to reach *any* assert-adjacent
panic machinery at all, which is at least as strong a claim as a `Pass`
under either other strategy, never weaker. The remaining terminals —
`UnwindResume`, `UnwindTerminate`, `Unreachable` — are `Edge::None`
unconditionally in `classify_terminator`, with no strategy check at all
(ADR 0003; `Unreachable` lowers to no code, so only the other two are a scope
exclusion); that scope exclusion is identical in every mode, including the
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
