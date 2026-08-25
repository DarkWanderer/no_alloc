# ADR 0003: Reject unresolved call edges, never assume them benign

## Status

Accepted, amended 2026-08-23 and 2026-08-25.

## Decision

An edge that represents a possible call but cannot be resolved to one concrete
MIR body is rejected. Rejection is not path-sensitive. This covers function
pointers, virtual dispatch, foreign callees without MIR, and inline assembly.

MIR control-flow terminators that do not call anything in MIR are treated as
safe — but here "safe" means "out of scope for this guarantee", not "proven
free of allocation". The guarantee is over a root's non-panicking execution
paths only; `README.md`'s "Guarantee and limitations" states that scope
explicitly. The exhaustive classification is:

| Terminator | Classification |
|---|---|
| `Call`, `TailCall` | Traverse a resolved instance; otherwise reject. A callee whose operand *type* is `FnDef` is resolved (ADR 0007); an intrinsic is classified against the table in ADR 0005 |
| `Drop` | Traverse required drop glue; no-op drop is safe |
| `InlineAsm` | Reject as opaque |
| `Assert` | Out of scope (not traversed) under a non-unwinding panic strategy (`panic=abort`); reject under `panic=unwind` |
| `Goto`, `SwitchInt`, `Return` | Safe control flow |
| `Unreachable`, `UnwindResume`, `UnwindTerminate` | Out of scope: no MIR callee, but each lowers to a real call (the panic runtime, or a foreign unwind-support routine) that this traversal does not follow |
| terminate-on-unwind action | Same as `UnwindTerminate`: out of scope, not a synthesized call edge |
| pre-lowering coroutine/false-edge forms | Reject as unexpected at this analysis stage |

The checker uses `Session::panic_strategy().unwinds()` as its sole signal for
`Assert`, and that is true of both non-unwinding strategies. Under plain
`panic=abort`, a failing assertion still calls `core::panicking::panic_fmt`
and the `#[panic_handler]` — which, with `std`, formats and prints, i.e.
allocates — before the process aborts; under `-Cpanic=immediate-abort` it
compiles to a bare `abort()`. The checker allows both identically, which is
deliberate: the process is aborting either way, so both are covered by the
same scope exclusion (panic paths are out of the guarantee). `panic=unwind`
rejects the terminator outright instead of attempting to model the panic
runtime.

(Amended 2026-08-25: this ADR previously described the immediate-abort case
as a std *feature*, `-Zbuild-std-features=panic_immediate_abort`. On the
pinned nightly it is a real panic strategy and asking for the old feature is
a `compile_error!`. `cargo no-alloc --immediate-abort` supplies the strategy
— see ADR 0006, which also explains why that mode needs no scope exclusion
here at all.)

## Consequences

Future `TerminatorKind` variants fail the exhaustive match until explicitly
classified. False positives from opaque calls are accepted in preference to a
false proof. Terminal unwind mechanics and `Assert` under a non-unwinding
strategy are not proof of "no allocation on this path" — they are excluded
from the guarantee's scope because they are codegen-time calls with no MIR
terminator to follow, and `README.md` states that exclusion explicitly. A
`--strict-panics` mode that resolves the panic lang items and traverses them
as real edges would close this gap for plain `panic=abort`; it is not
implemented. `--immediate-abort` (ADR 0006) closes it a different way, by
removing the panic runtime so those edges have nothing left to hide.
