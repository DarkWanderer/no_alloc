# ADR 0007: Resolve shim instances and function-item callees

## Status

Accepted 2026-08-25.

## Context

Two mechanical details of the traversal were rejecting call edges that are
perfectly resolvable, and both of them fire on the same code shape: a
callback that reaches its call site as a function *item* rather than as a
closure. That shape is everywhere in the standard library —
`Iterator::max` is `max_by(Ord::cmp)`, `min_by_key` wraps the key function,
`Iterator::last` reborrows its fold closure as `&mut F`.

1. **Body availability was asked about the wrong thing.** The traversal
   rejected any callee where `tcx.is_mir_available(instance.def_id())` was
   false. For a compiler-generated shim — `<&mut F as FnOnce>::call_once`,
   `FnMut::call_mut`, drop glue, clone shims — that `DefId` is the trait
   method the shim stands for, which has no body, while the shim instance
   itself has one that `instance_mir` builds on demand.

2. **Callees were classified by MIR form, not by type.**
   `Operand::const_fn_def` answers "is this callee a MIR constant with
   `FnDef` type", so a callee that arrives as a move out of a local was
   rejected as "a function pointer" even when its type named exactly one
   function. Shim bodies call the function they were built for exactly that
   way.

Together these made `buf.iter().max()` unverifiable — not because anything on
the path allocates, but because the traversal stopped one step before the
`Ord::cmp` it could see perfectly well.

## Decision

**Body availability is a property of the `InstanceKind`.** rustc documents
exactly three kinds as having no callable MIR of their own: `Intrinsic`,
`LlvmIntrinsic`, and `Virtual`. `Intrinsic` is classified against the table
in ADR 0005; the other two reject. `Item` is the only kind for which
`is_mir_available` (and `is_foreign_item`) is the right question. Every
`Shim` kind has a generated body and is traversed. The match is exhaustive,
so a new `InstanceKind` in a future toolchain fails to compile until someone
classifies it.

**A call edge is resolved from the callee operand's type.** If it is
`ty::FnDef`, it names one function and resolves; anything else is a genuine
function pointer and rejects. Generic arguments that still carry bound
variables reject rather than being unwrapped — that should not happen from a
monomorphized root, and a surprise there should be a finding, not an ICE.

## Consequences

Measured on [`examples/iterators`](../examples/iterators) under
`--immediate-abort`, passing roots go from 29 to 33 of 35: `max`,
`min_by_key`, `flat_map(..).count()`, and `.scan(..).last()` become
checkable. The other 20 fixtures in `tests/ui` are unchanged, including
`fnptr_reject`, `dyn_reject`, `closure_call`, and the drop-glue cases —
this widens what can be *followed*, not what is assumed.

Nothing here weakens ADR 0003. A shim that wraps a virtual call still lands
on the `Virtual` instance and rejects there; a real `fn` pointer still
rejects, now with the reason attached to the operand whose type actually is
a pointer. `slice::sort_unstable` is the worked example of that: it dispatches
its partition implementation through a `fn` pointer, so it rejects on the
merits (`tests/ui/iterator_immediate_abort::sorted`), where before it
rejected for the wrong reason at a shim.

The traversal now walks more std internals per root, which costs time and
makes rejection chains longer. That is the intended trade: a chain that ends
at the fn pointer std actually uses is worth more than one that ends at a
shim the compiler generated.
