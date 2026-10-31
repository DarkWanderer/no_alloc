# ADR 0005: Classify intrinsics against a leaf table instead of rejecting them

## Status

Accepted 2026-08-25, amended 2026-10-21.

## Context

Until now every `InstanceKind::Intrinsic` callee was rejected, sharing the
"no statically available MIR body for this callee" path with foreign items
and `dyn` dispatch. That reading conflates two different situations. A
foreign callee is *unresolved*: the traversal does not know what runs. An
intrinsic is *fully resolved* — `Instance::try_resolve` named it exactly —
and what it lacks is a MIR body, because its body is the code the backend
emits for it. ADR 0003 asks the checker to reject what it cannot resolve;
it does not ask it to reject what it has resolved and can classify.

The practical cost of the old behaviour was that no iterator was checkable.
Under `-Cpanic=immediate-abort` (ADR 0006) every panic path in the slice
iterator internals ends in `core::intrinsics::abort`, and the hint around an
overflow check ends in `core::intrinsics::cold_path`. Both were rejected, so
all twenty iterator patterns in `docs/iterators.md` were rejected, and none
of them at anything that allocates.

## Decision

An intrinsic callee is classified against a table of intrinsics whose
lowering provably contains no call into Rust code, and is therefore terminal
rather than unresolved. The table lives in
`crates/no_alloc_report/src/intrinsic_table.rs` (a pure function of a name,
so it is unit-testable on stable, like `parse_root_spec`).

- The table is an **allowlist**. An intrinsic that is not named in it still
  rejects, with a message that names it — so a new intrinsic in a future
  toolchain rejects until someone has classified it, which is the direction
  of failure ADR 0003 asks for.
- Entries are matched on `ty::IntrinsicDef::name`, the compiler's own
  dispatch key for the intrinsic. This is the intrinsic's identity, not a
  guess derived from a linker symbol, and so is not the symbol-name matching
  ADR 0003 rules out for allocator detection (`leaf.rs` still uses
  `CodegenFnAttrFlags` and nothing else).
- Deliberately excluded: `catch_unwind`, `const_eval_select`,
  `contract_check_requires`, `contract_check_ensures`, `autodiff`,
  `offload` — each runs a function supplied by its caller, which is exactly
  the case the traversal cannot see through — plus `const_allocate`,
  `const_deallocate`, `const_make_global`, and the SIMD, GPU, `va_*`, and
  `rustc_peek` families, which this table has not audited.
- One family cannot be settled by name at all. `assert_inhabited`,
  `assert_zero_valid` and `assert_mem_uninitialized_valid` compile to
  nothing for a type that meets the requirement and to a call to the
  `panic_nounwind` lang item for one that does not
  (`rustc_codegen_ssa`'s `codegen_panic_intrinsic`) — a Rust function
  synthesized after MIR, which the traversal would never see, and which
  outside `-Cpanic=immediate-abort` reaches the allocating panic handler.
  They are therefore absent from the table and classified by the traversal
  per instantiation, using the same `check_validity_requirement` query
  codegen uses: requirement holds, terminal; requirement fails or the layout
  is unknown, rejected. That is per-instantiation classification in ADR
  0001's sense, not a name-keyed leaf.

The allocator leaf check still runs before the intrinsic check, for the same
ordering reason `leaf.rs` documents: a classification must never be able to
mask a real allocator terminal.

## Consequences

Iterators become checkable under the mode in ADR 0006 (19 of the 20 patterns
in `docs/iterators.md` pass; the twentieth fails for an unrelated reason).
Ordinary intrinsic-backed code is checkable in any mode: `f32::sqrt` and
`u32::count_ones` no longer reject (`tests/ui/intrinsic_leaf`).

The table is a soundness surface: each entry asserts that the compiler's
lowering of that intrinsic cannot reach the allocator — including that it
emits no Rust call, which is subtler than it looks (the validity assertions
above were in an earlier draft of this table and had to come out). Adding an entry is a
claim to be checked against that toolchain's lowering, not a convenience for
making a test pass — an intrinsic that takes a function and calls it must
never be added. `tests/ui/intrinsic_reject` pins the rejecting half,
`intrinsic_table.rs`'s unit tests pin the excluded names, and
`tests/ui/validity_assert` plus `strict_iterator::search` pin the
two sides of the per-instantiation family.

Because the table is keyed on names from a pinned nightly, a toolchain bump
must re-check it: a renamed intrinsic silently becomes unclassified (safe:
it rejects), while a *reused* name whose lowering changed would not be
caught by anything here.

### Fallback bodies (amended 2026-10-21)

The table vouches for the backend's lowering, and the backend does not
always lower. An intrinsic with a Rust fallback body is codegen'd as a call
to that body whenever the backend has no arm for it, and
`typed_swap_nonoverlapping` (`mem::swap`) is deliberately lowered that way
for a non-SSA type when optimizing (`IntrinsicResult::Fallback` in
`rustc_codegen_ssa`/`rustc_codegen_llvm`). The fallback is ordinary MIR,
and before this amendment it was never walked: a root swapping a large
array at `opt-level = 1` passed while its code called
`ptr::swap_nonoverlapping` and the UB-check panic path inside it.

The traversal now also walks the fallback body of a table intrinsic
whenever rustc's own mono collector would emit it (`visit_instance_use`: a
fallback exists, and the backend has not declared the intrinsic in
`replaced_intrinsics`, unless `-Zforce-intrinsic-fallback` is set). The one
per-instantiation choice in `codegen_intrinsic_call`,
`typed_swap_nonoverlapping` swapping in place for an SSA-standalone type or
at `opt-level = 0`, is mirrored so `mem::swap` is not rejected for a body
codegen never calls. `tests/ui/intrinsic_fallback` pins both sides.

### Whose session decides the swap lowering (amended 2026-10-24)

`typed_swap_nonoverlapping` is lowered in place at `opt-level = 0`, but
"opt-level" is the session of the crate that codegens the *caller*, and
opt-level is set per package profile. The checker runs in one crate's
session, so reading its opt-level for a body codegened in a dependency
(non-generic, MIR available cross-crate, built at a higher opt-level) passed
code whose object code calls the fallback. The rule is now per caller: the
`OptLevel::No` shortcut applies only if the caller is codegened in this
crate. That is rustc's own `should_codegen_locally`, applied along the walk
the way the collector applies it: a root is local if rustc would codegen it in this crate
(`should_codegen_locally`), which includes every local definition (reachable
or not, so a private never-called root is still judged at this crate's
opt-level); a callee is local only if its caller is and
`should_codegen_locally(callee)` holds, which is false for an upstream
non-generic and for an upstream-shared generic instantiation. The query alone
is not enough, since an `#[inline]` generic like `mem::swap::<T>` passes it
yet is emitted only where it is reached. The SSA-standalone test is layout-based and
session-independent, so it is unchanged. Any other caller walks the
fallback. `tests/ui/intrinsic_fallback_upstream_opt` pins both the upstream
caller and the dead local root.

For a non-generic dependency function, whether it is codegened upstream or
copied into the downstream crate is decided by rustc's cross-crate-inlinable
inference, which depends on the environment (for example incremental
compilation), so it is not a stable fixture. The deterministic case is an
upstream-shared generic instantiation: a dependency at `opt-level = 1` keeps
share-generics on (it is off only for 2/3), exports its non-`#[inline]`
`swap_generic::<[u64; 16]>`, and lowers the `mem::swap` inside it through the
fallback because it is optimizing. The root crate at opt-level 0 reuses that
instantiation (`should_codegen_locally` is false), so the in-place shortcut
must not apply. `tests/ui/intrinsic_fallback_upstream_opt` pins this.

Known precision cost: with share-generics on at `opt-level = 0`, if any linked
dependency built at `opt-level = 0` instantiates the same `mem::swap::<T>`,
`should_codegen_locally` reports the instance as upstream-shared, the callee
becomes non-local, and the fallback is walked. A local root doing `mem::swap`
on a non-SSA type is then Rejected even though the exporting crate lowered it
in place. This is conservative and acceptable under ADR 0003;
`tests/ui/shared_generic_swap_conservative` pins it as a known false
rejection, not as desired behaviour.

(Amended 2026-10-31, ADR 0008.) On the pinned sysroot the only edge in
`ptr::swap_nonoverlapping`, the fallback body, that can reject is its
UB-check panic. In the default mode that panic is out of scope, and on the
target under `--strict` it ends in `abort`, so walking the fallback and
skipping it now give the same `Pass`. `intrinsic_fallback`,
`intrinsic_fallback_upstream_opt` and `shared_generic_swap_conservative`
therefore no longer observe the walk, the per-caller rule, or the false
rejection; each now only pins that nothing else in the fallback rejects or
allocates. What still observes them is a host unit under `--strict`, which
compiles under `unwind` against the precompiled host sysroot, where the
panic is in scope and has no MIR: `strict_host_panic_reject` pins the
fallback walk (`host_swap_large` rejects, `host_swap_scalar` passes), and
`strict_host_fallback_upstream_opt` pins the per-caller rule (a build script
at opt-level 0 reusing an opt-level-1 build-dependency's
`swap_generic::<[u64; 16]>` rejects inside the fallback, while its dead
local root passes). Cargo's `[profile.dev.package.<dep>]` override takes
precedence over `build-override`, which is what puts that build-dependency at
opt-level 1 on the host; verified from the `-v` rustc invocation. The known
false rejection has no host twin and is currently unobserved.

### What the table does not cover, and neither does the tool

Some of these lowerings emit a call to a *named* symbol — `powf` for
`llvm.pow.f32`, `memcpy` for a large copy, `__addtf3` for `f128` arithmetic.
A program can define that symbol itself, in Rust, and allocate in it:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn powf(_x: f32, _y: f32) -> f32 { *Box::new(2.0) }
```

A root calling `f32::powf` then passes, wrongly. This is a genuine limit,
and it is worth being precise about whose limit it is: **it is not created
by this table**. Verified on the pinned toolchain, a `#[no_alloc]` function
whose entire body is `*big` for a 512-byte struct compiles to
`call void @llvm.memcpy.p0.p0.i64(..., i64 512, ...)` — the same
interposable symbol, reached from a function containing no intrinsic, no
`Call` terminator, and nothing in MIR for any traversal to walk. Both
functions pass today.

So the boundary of the guarantee is MIR: the tool sees the calls the
compiler puts in MIR, not the ones the backend synthesizes from it. Making
the float and bulk-memory intrinsics reject would not restore soundness
against a hostile `#[no_mangle]` — it would reject `x.sqrt()` while still
passing the struct move next to it, which is a worse place to stand than
stating the boundary plainly. It is stated in `docs/guarantees.md`.

If the project would rather pay that cost — rejecting `sqrt`, `powf`,
`compare_bytes` and the rest of the libcall-backed entries, and accepting
that DSP-shaped code stops being checkable — the change is mechanical:
delete `is_float_arithmetic` and the bulk-memory names from this table.
