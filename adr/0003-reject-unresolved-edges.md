# ADR 0003: Reject unresolved call edges, never assume them benign

## Status

Accepted, amended 2026-08-23, 2026-08-25, and 2026-09-03.

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

Reaching a callee's body is a separate exhaustive decision. It is classified
from the resolved `InstanceKind` and, for compiler-generated shims, every
`ShimKind`; asking whether `instance.def_id()` has MIR answers the wrong
question for a shim, whose MIR is synthesized on demand.

| `InstanceKind` / `ShimKind` | Classification |
|---|---|
| `Item` | Traverse if MIR is available and the item is not foreign; otherwise reject |
| `Intrinsic` | Classify against ADR 0005's audited intrinsic table; reject an unrecognized intrinsic |
| `LlvmIntrinsic` | Reject — no callable MIR exists |
| `Virtual` | Reject — the runtime-selected vtable callee is not statically known |
| `Shim(VTable)`, `Shim(Reify)`, `Shim(FnPtr)`, `Shim(ClosureOnce)` | Traverse — the synthesized body exposes its direct or indirect call to the ordinary call-edge classifier |
| `Shim(DropGlue)` | Traverse for concrete types; a `dyn` place is rejected by the `Drop` terminator before constructing its glue |
| `Shim(Clone)` | Traverse — the synthesized body contains either a trivial copy or the real per-field clone calls |
| `Shim(FnPtrAddr)` | Traverse — the synthesized body is a call-free pointer cast |
| `Shim(ThreadLocal)` | Reject — TLS access can lower to a runtime call with no MIR callee |
| `Shim(ConstructCoroutineInClosure)`, `Shim(FutureDropPoll)`, `Shim(AsyncDropGlue)`, `Shim(AsyncDropGlueCtor)` | Reject — the unstable synthesized bodies have not been audited |

Every shim arm above was checked against
`rustc_mir_transform::shim::make_shim` on the pinned nightly. Future
`InstanceKind` or `ShimKind` variants fail the exhaustive match until they are
classified explicitly.

### Non-terminator MIR (F3, 2026-09)

The traversal also audits statements. `Rvalue::ThreadLocalRef` is rejected:
rustc documents it as a runtime operation that executes code, and on some TLS
models it lowers to `__tls_get_addr` without a MIR callee the analysis can
follow. Every other `Rvalue` is call-free data movement or computation.
Non-diverging `Assume` and `CopyNonOverlapping` statement intrinsics are also
call-free, and the remaining statement kinds are compiler metadata or
storage/discriminant operations with no call edge. The same conservative TLS
decision applies to `ShimKind::ThreadLocal`; `thread_local_reject` covers the
same-crate statement form.

Demonstrating an allocator call through dynamic TLS would require a `cdylib`
loaded with `dlopen` and has not been done. The rejection does not rely on that
platform-specific path: the unmodeled runtime operation is sufficient under
the reject-don't-assume policy.

### Retrospective: the `dyn` drop-glue gap (F0, 2026-09)

`Instance::resolve_drop_glue(tcx, dyn Trait)` produces a synthesized
`DropGlue` body containing another `Drop` on the same `dyn` place. The old
traversal revisited the same instance and treated the cycle as complete,
while codegen actually calls the concrete destructor through the vtable's
drop slot. The `Drop` classifier now rejects a `dyn` place before constructing
that glue. The `drop_dyn_arena` fixture pins this behavior.

The checker uses `Session::panic_strategy().unwinds()` as its sole signal for
`Assert`, and that is true of both non-unwinding strategies. Under plain
`panic=abort`, a failing assertion still calls `core::panicking::panic_fmt`
and the `#[panic_handler]` — which, with `std`, formats and prints, i.e.
allocates — before the process aborts; under `-Cpanic=immediate-abort` it
compiles to a bare `abort()`. The checker allows both identically, which is
deliberate: the process is aborting either way, so both are covered by the
same terminator-shaped scope exclusion. `panic=unwind`
rejects the terminator outright instead of attempting to model the panic
runtime.

The exclusion is terminator-shaped, not panic-shaped. It covers only the
terminators in the table above that have no MIR callee to follow. An explicit
panic (`panic!()`, `.unwrap()`, `.expect()`, and similar code) lowers to an
ordinary `Call` terminator and is in scope; a foreign or bodiless panic-runtime
callee rejects like any other unresolved call. Thus indexing (`Assert`) can
pass under `panic=abort` while `get(index).unwrap()` (`Call`) rejects under the
same profile. The `explicit_panic` and `panic_abort` fixtures pin the boundary.

(Amended 2026-08-25: this ADR previously described the immediate-abort case
as a std *feature*, `-Zbuild-std-features=panic_immediate_abort`. On the
pinned nightly it is a real panic strategy and asking for the old feature is
a `compile_error!`. `cargo no-alloc --immediate-abort` supplies the strategy
— see ADR 0006, which also explains why that mode needs no scope exclusion
here at all.)

## Consequences

Future `TerminatorKind`, `InstanceKind`, and `ShimKind` variants fail their
exhaustive matches until explicitly classified. False positives from opaque
calls are accepted in preference to a false proof. Terminal unwind mechanics
and `Assert` under a non-unwinding strategy are not proof of "no allocation on
this path" — they are excluded from the guarantee's scope because they are
codegen-time calls with no MIR terminator to follow, and `README.md` states
that exclusion explicitly. A `--strict-panics` mode that resolves the panic
lang items and traverses them as real edges would close this gap for plain
`panic=abort`; it is not implemented. `--immediate-abort` (ADR 0006) closes
it a different way, by removing the panic runtime so those edges have nothing
left to hide.
