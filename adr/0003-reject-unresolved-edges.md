# ADR 0003: Reject unresolved call edges, never assume them benign

## Status

Accepted, amended 2026-08-23.

## Decision

An edge that represents a possible call but cannot be resolved to one concrete
MIR body is rejected. Rejection is not path-sensitive. This covers function
pointers, virtual dispatch, foreign callees without MIR, and inline assembly.

MIR control-flow terminators that do not call anything are safe. The exhaustive
classification is:

| Terminator | Classification |
|---|---|
| `Call`, `TailCall` | Traverse a resolved instance; otherwise reject |
| `Drop` | Traverse required drop glue; no-op drop is safe |
| `InlineAsm` | Reject as opaque |
| `Assert` | Safe only when rustc reports a non-unwinding panic strategy; otherwise reject |
| `Goto`, `SwitchInt`, `Return` | Safe control flow |
| `Unreachable`, `UnwindResume`, `UnwindTerminate` | Safe terminal control flow |
| terminate-on-unwind action | Safe terminal control flow; it is not a synthesized call edge |
| pre-lowering coroutine/false-edge forms | Reject as unexpected at this analysis stage |

The checker uses `Session::panic_strategy().unwinds()` as the authoritative
fact for assertions. `panic=abort` and immediate-abort strategies therefore
allow assertion terminators; an unwinding strategy rejects them without trying
to model the panic runtime.

## Consequences

Future `TerminatorKind` variants fail the exhaustive match until explicitly
classified. False positives from opaque calls are accepted in preference to a
false proof. Terminal unwind mechanics are not mislabeled as calls merely
because code generation later materializes runtime support for them.
