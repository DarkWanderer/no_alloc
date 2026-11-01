# ADR 0003: Reject unresolved call edges, never assume them benign

## Status

Accepted, amended 2026-08-23, 2026-08-25, 2026-09-03, 2026-10-21, 2026-10-30, and 2026-10-31.

## Decision

An edge that represents a possible call but cannot be resolved to one concrete
MIR body is rejected. Rejection is not path-sensitive. This covers function
pointers, virtual dispatch, foreign callees without MIR, and inline assembly.

MIR control-flow terminators that do not call anything in MIR are treated as
safe — but here "safe" means "out of scope for this guarantee", not "proven
free of allocation". The guarantee is over a root's non-panicking execution
paths only; `docs/guarantees.md` states that scope
explicitly. The exhaustive classification is:

| Terminator | Classification |
|---|---|
| `Call`, `TailCall` | Traverse a resolved instance; otherwise reject. A callee whose operand *type* is `FnDef` is resolved (ADR 0007); an intrinsic is classified against the table in ADR 0005 |
| `Drop` | Traverse required drop glue; no-op drop is safe |
| `InlineAsm` | Reject as opaque |
| `Assert` | Default mode: out of scope under every panic strategy (ADR 0008). `--strict`: out of scope (not traversed) under `panic=abort`; traversed into the panic lang item codegen calls under `-Cpanic=immediate-abort` (ADR 0006); reject under `panic=unwind` |
| `Goto`, `SwitchInt`, `Return` | Safe control flow |
| `Unreachable` | Safe: codegens to a bare LLVM `unreachable`, no code |
| `UnwindResume`, `UnwindTerminate` | Out of scope: no MIR callee, but each lowers to a real call (a foreign unwind-support routine, or a nounwind panic-handler call) that this traversal does not follow |
| terminate-on-unwind action | Same as `UnwindTerminate`: out of scope, not a synthesized call edge |
| pre-lowering coroutine/false-edge forms | Reject as unexpected at this analysis stage |

Reaching a callee's body is a separate exhaustive decision. It is classified
from the resolved `InstanceKind` and, for compiler-generated shims, every
`ShimKind`; asking whether `instance.def_id()` has MIR answers the wrong
question for a shim, whose MIR is synthesized on demand.

| `InstanceKind` / `ShimKind` | Classification |
|---|---|
| `Item` | Traverse if MIR is available and the item is not foreign; otherwise reject. Also reject when the item has explicit `linkage`/`import_linkage` codegen attributes (amended 2026-10-30): the linker may bind the symbol to a different definition (weak linkage), so this MIR is not evidence of what runs |
| `Intrinsic` | Classify against ADR 0005's audited intrinsic table, and walk its fallback body whenever codegen may call it; reject an unrecognized intrinsic |
| `LlvmIntrinsic` | Reject — no callable MIR exists |
| `Virtual` | Reject — the runtime-selected vtable callee is not statically known |
| `Shim(VTable)`, `Shim(Reify)`, `Shim(FnPtr)`, `Shim(ClosureOnce)` | Traverse — the synthesized body exposes its direct or indirect call to the ordinary call-edge classifier |
| `Shim(DropGlue)` | Traverse for concrete types. A `Drop` terminator on a `dyn` place is rejected before constructing glue; an explicit `ptr::drop_in_place::<dyn T>` call does construct it (`DropGlue(_, Some(dyn T))`) and is rejected on the shim body's residual `Drop` of the `dyn` place |
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

**Extern-type tail projection (amended 2026-10-30).** A `ProjectionElem::Field`
into a type whose `struct_tail_for_codegen` is `ty::Foreign` is rejected, as a
place-level rejection alongside `ThreadLocalRef`. Codegen's `project_field`
computes the unsized tail's size through `size_and_align_of_dst`, whose
`Foreign` arm emits a `panic_nounwind` lang-item call that appears in no MIR
body. The rule does not mirror the offset condition under which codegen takes
that path; any such projection rejects. It is recorded as a pending rejection,
so an allocator violation elsewhere in the same body still wins. Pinned by
`tests/ui/extern_type_tail`; the linkage rule above by
`tests/ui/weak_linkage_reject`.

**Async fn roots (amended 2026-10-30).** An `async fn` has MIR only for its
future constructor; the body that runs is a separate coroutine. A marked or
`--root`-selected `async fn` is a selection error rather than a pass on the
constructor (`tests/ui/async_root_reject`).

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
that glue. An explicit `ptr::drop_in_place::<dyn T>` call does construct the
glue, and is rejected on the glue body's own `Drop` of the `dyn` place. The
`drop_dyn_arena` fixture pins this behavior.

Under plain `panic=abort`, a failing assertion still calls
`core::panicking::panic_fmt` and the `#[panic_handler]` — which, with `std`,
formats and prints, i.e. allocates — before the process aborts. The checker
allows that `Assert` as a terminator-shaped scope exclusion. Under
`-Cpanic=immediate-abort` there is no exclusion: codegen lowers a failing
assertion to a call to a core panic lang item (`panic_bounds_check`,
`panic_const_add_overflow`, ...), chosen as `codegen_assert_terminator`
chooses it, and the traversal resolves and walks that call (amended
2026-10-21; it was previously allowed on the assumption that the lang item
is `abort` in that mode, which is now checked instead). `panic=unwind`
rejects the terminator outright instead of attempting to model the panic
runtime.

The exclusion is terminator-shaped, not panic-shaped. It covers only the
terminators in the table above that have no MIR callee to follow. An explicit
panic (`panic!()`, `.unwrap()`, `.expect()`, and similar code) lowers to an
ordinary `Call` terminator and is in scope; a foreign or bodiless panic-runtime
callee rejects like any other unresolved call. Thus indexing (`Assert`) can
pass under `panic=abort` while `get(index).unwrap()` (`Call`) rejects under the
same profile. The `explicit_panic` and `panic_abort` fixtures pin the boundary.

(Amended 2026-10-31, ADR 0008: the two paragraphs above now describe
`--strict` only. In the default mode the exclusion is panic-shaped after all:
an `Assert` is out of scope under every strategy, and a call to a sysroot
panic entry point named in `no_alloc_report`'s panic table ends the walk. The
`explicit_panic` and `panic_abort` fixtures now pin the default-mode side, both
passing; the strict-mode boundary is pinned by `strict_host_panic_reject`
(`unwrap` rejects at `unwrap_failed`, indexing at its `Assert` under
`unwind`). A
bodiless callee that is not in that table still rejects.)

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
codegen-time calls with no MIR terminator to follow, and `docs/guarantees.md` states
that exclusion explicitly. A `--strict-panics` mode that resolves the panic
lang items and traverses them as real edges would close this gap for plain
`panic=abort`; it is not implemented. `--immediate-abort` (ADR 0006) closes
it a different way, by removing the panic runtime so those edges have nothing
left to hide. (Amended 2026-10-31: ADR 0008 makes that the opt-in mode,
renamed `--strict`, and makes panic paths out of scope by default.)
