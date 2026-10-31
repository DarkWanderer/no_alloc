# ADR 0008: Panic paths are out of scope by default; `--strict` checks them

## Status

Accepted 2026-10-31. Amends ADR 0003 (the `Assert` row, and the
`--strict-panics` mode it left unimplemented) and ADR 0006 (its flag is now
`--strict`, and it is no longer the only mode in which realistic code passes).

## Context

Before this decision the checker had one useful mode, and it was spelled after
a compiler mechanism. The default run put panic paths *in* scope but could not
follow them: an `Assert` rejected under `panic = "unwind"`, and every explicit
panic (`.unwrap()`, `panic!()`, the UB-check precondition inside
`slice::Iter::next`) is a `Call` into a sysroot function the precompiled
standard library ships no MIR for, so it rejected under any profile. Measured
on `examples/iterators`, the default and `panic = "abort"` passed none of the
35 roots, and every rejection was inside the panic machinery, not at anything
that allocates (ADR 0006).

The only way out was `--immediate-abort`, which rebuilt the standard library
with `-Cpanic=immediate-abort` so the panic path could be walked to
`intrinsics::abort`. That mode is sound and worth keeping, but as the only
working mode it had three costs:

- A compiler mechanism leaked into the interface. Users had to learn a panic
  strategy to get any answer, and the flag's name described how it worked,
  not what it promised.
- Nothing realistic was checkable by default. Any root that indexes, unwraps,
  or iterates rejected, so the default run's answer to almost every real
  question was "rejected", which teaches users to ignore it.
- `-- test` was unusable. Cargo builds tests under `panic=unwind` whatever
  the profile says, and rustc will not build a libtest harness under an abort
  strategy, so neither mode could check code compiled as tests.

What most users of a no-allocation checker are asking is "does the code that
runs when nothing goes wrong allocate?". A panic in a real-time or embedded
context is already a failure; whether the failure path allocates is a
separate, stronger question.

## Decision

**The default guarantee is "no allocation on any non-panicking path".** The
traversal is given an explicit mode, `no_alloc_report::PanicPaths`, and
records it in every root's `environment` (`panic_paths`, report schema
version 3):

- `out_of_scope` (default). The walk stops at a call to a standard-library
  panic entry point, and an `Assert` terminator is not followed under any
  panic strategy, `unwind` included. That is the same reasoning as ADR 0003's
  existing `panic=abort` carve-out for `Assert`, applied to the definition of
  the guarantee rather than to one kind of terminator: a failing assertion is a
  panic by construction, so whatever it calls is on a panicking path.
- `checked` (`--strict`). Exactly the previous immediate-abort mode: the crate
  and the standard library are rebuilt with `-Zunstable-options
  -Cpanic=immediate-abort`, `--build-std` is implied, the entry-point table is
  not consulted, and `Assert` is classified per panic strategy as before
  (`unwind` rejects, `abort` is out of scope, `immediate-abort` walks the lang
  item). `--immediate-abort` is removed, not aliased: the old spelling is an
  unknown option.

The mode reaches the driver as `NO_ALLOC_STRICT`, set on every run like the
other `NO_ALLOC_*` variables (so Cargo's `[env]` table cannot fill it, and
`[env]` keys with that prefix are already refused), and derived from the same
`StdMode` value that injects the panic strategy, so "checked panic paths over
a sysroot that still has its panic runtime" is not a representable
invocation.

**Panic entry points are an allowlist keyed by defining path.**
`crates/no_alloc_report/src/panic_table.rs` names each sysroot function every
path through which ends in a panic (`core::panicking::panic_fmt`,
`core::option::unwrap_failed`, `core::panicking::panic_const::*`,
`alloc::raw_vec::capacity_overflow`, `std::panicking::begin_panic`, ...), each
checked against the pinned sysroot's source. The key is the crate-qualified
*defining* path built from the `DefPath`, not `def_path_str`, which prints the
visible re-export (`std::option::unwrap_failed`), omits the crate for a local
item, and varies with printing context. A crate is accepted as `core`, `alloc`
or `std` only if it defines that library's anchor lang item (`panic_fmt`,
`owned_box`, `begin_panic`), so a user module path cannot impersonate one
(`tests/ui/panic_table_user_path`). The residual is a crate that *is* the
anchor: in a graph with no real `std`/`alloc`, a crate named `std` or `alloc` (or a
`#![no_core]` crate named `core`) that defines the anchor lang item itself
would be accepted. That needs `#![feature(lang_items)]` and a crate name
Cargo normally refuses, so it is a deliberate act, not an accident. Only a plain `InstanceKind::Item` matches. The table is checked after the
allocator leaf and before body resolution, for the same ordering reason as
the intrinsic table (ADR 0005).

Two alternatives were rejected:

- **Panic lang items only.** They cover what codegen calls for an `Assert`
  but not `unwrap_failed`, `expect_failed`, `slice_index_fail`, or the RefCell
  borrow failures, which are ordinary functions. Iterators and `unwrap` would
  still reject at those, and the default would still check nothing realistic.
- **Any diverging sysroot function.** `process::abort`, `process::exit` and
  `handle_alloc_error` diverge without panicking. A path that allocates and
  then aborts is a non-panicking path the default guarantee covers, and
  `handle_alloc_error` is what an allocation *failure* calls, so excusing it
  would excuse the allocator. Divergence is a type-level fact; "is a panic" is
  a claim about the body, which is why it has to be an audited list.

Everything else is unchanged. An unresolved call still rejects, a `-> !`
function not in the table is walked or rejected like any other callee
(`tests/ui/panic_table_unknown_rejects`), and the validity-assertion
intrinsics and extern-type tail projections, whose codegen emits a panic call
no MIR edge names, still reject in both modes.

## Consequences

The default claim is weaker, and has to be stated precisely: a `Pass` with
`panic_paths: "out_of_scope"` says that no allocator entry point is reachable
from the root except through a call to a listed panic entry point or the
lowering of a failing `Assert`. It says nothing about what a panic does (the
`#[panic_handler]` with `std` formats and prints; `begin_panic` boxes its
payload), and that includes user code the panic machinery calls: a `no_std`
crate's own `#[panic_handler]` and a `std::panic::set_hook` hook are
excluded too. It holds under the build's own panic strategy, `unwind`
included. What is still in scope on a panicking path: everything walked code
does *before* calling the entry point (a `String` built for a panic message
is a violation, `tests/ui/panic_message_allocates`) and the unwinding
cleanup of every walked body, the root's and every callee's: the traversal
visits every basic block of each body it walks, cleanup blocks and their
drop glue included.
Catching a panic needs `catch_unwind`, which is not in the intrinsic table and
rejects.

`cargo no-alloc -- test` is usable by default: the test profile's `unwind`
no longer rejects every `Assert`. `--strict` still cannot be combined with it.

`--strict` keeps the strongest claim, with no panic-path carve-out, at the
same cost as before: a full sysroot rebuild on every run. Measured on
`examples/iterators`, the default now passes 33 of 35 roots under both
`panic = "unwind"` and `panic = "abort"` (previously 0 and 0), the same 33
`--strict` passes; the other two are a real allocation and a real function
pointer in all three (`docs/iterators.md`).

The table is a soundness surface in the same way as the intrinsic table. An
entry asserts "every path through this function panics" about one pinned
sysroot. A toolchain bump must re-check it: a renamed function silently
leaves the table (safe: it is walked or rejects), while a function whose body
stops panicking on some path would not be caught by anything here.
