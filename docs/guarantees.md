# Guarantee and limitations

Analysis starts from each selected monomorphized `Instance`, follows resolved
calls, tail calls, compiler-generated shims, and drop glue, and detects
allocator, deallocator, reallocator, and zeroed-allocator terminals.
Function pointers, virtual dispatch, foreign calls without MIR, inline
assembly, and intrinsics outside the non-allocating intrinsic table are
rejected ([ADR 0005](../adr/0005-intrinsic-leaf-classification.md),
[ADR 0007](../adr/0007-shim-and-fn-item-resolution.md)). A
`Violation` chain names *a* reachable allocator path, not necessarily the one
actually taken at runtime — the DFS reports whichever call sequence it finds
first among several that may exist; this is correct for a reachability
claim, but don't read the chain as "this is the line that allocates" without
checking.

A `mem::swap` of a non-SSA type in a local root can be rejected when a linked
dependency built at the same opt-level instantiates the same `mem::swap::<T>`
(share-generics makes it upstream-shared); this is a conservative false
rejection ([ADR 0005](../adr/0005-intrinsic-leaf-classification.md)).

**The panic exclusion is drawn by terminator shape, not by whether a path
panics.**
`UnwindResume` and `UnwindTerminate` have no MIR callee to follow, so the
traversal treats them as terminal control flow rather than calls — but each
lowers to a real call during codegen (a foreign `_Unwind_Resume`, or a
nounwind panic-handler call) that the traversal never walks. `Unreachable`
codegens to a bare LLVM `unreachable` and contains no code at all. An
`Assert` terminator (the implicit checks inserted for array bounds,
arithmetic overflow, and similar cases) is allowed only under a non-unwinding
panic strategy (`panic=abort`), and that is *not* because the panic runtime
is unreachable there: a failing assertion under plain `panic=abort` still
calls `panic_fmt` and the `#[panic_handler]` (which, with `std`, formats and
prints — i.e. allocates) before the process aborts. It is allowed because
allocation reachable only through a panic, an unwind-terminate, or an
unwind-resume path is out of scope for this guarantee, not because that
allocation has been ruled out. An explicit `panic!()`, `.unwrap()`, or `.expect()` has a
different MIR shape: it is an ordinary `Call` terminator and remains in scope,
so an unresolved panic-runtime callee rejects even under `panic=abort`. Under
`panic=unwind`, `Assert` is rejected outright instead, on the usual
reject-don't-assume grounds — see [ADR 0003](../adr/0003-reject-unresolved-edges.md)
for the full terminator classification and rationale.

**`--immediate-abort` is the exception, and the stronger guarantee.** It
compiles the crate and the standard library with `-Cpanic=immediate-abort`,
where every panic — including the panic lang item a failing `Assert` calls —
lowers to a bare `abort()` with no handler call. The traversal then walks
panic paths, `Assert`'s included, as ordinary edges and
finds `core::intrinsics::abort` at the end of them, so in that mode there is
no panic-path carve-out at all. It costs a full sysroot rebuild per run. See
[ADR 0006](../adr/0006-immediate-abort-checking-mode.md).

In practice this means most indexing and arithmetic in debug builds — which
lower to an `Assert` — reject under `panic=unwind` (Rust's default), and
`panic = "abort"` in `[profile.*]` is the minimum for anything to pass. It is
not enough for iterators: the standard library reaches the panic machinery
through ordinary calls as well as `Assert` terminators, and those reject
until the sysroot is rebuilt. Measured over 35 iterator patterns, `abort`
passes none of them and `--immediate-abort` passes 33 — see
[`docs/iterators.md`](iterators.md).

**The boundary is MIR, not the machine code.** The analysis follows the
calls the compiler puts in MIR; it does not see the ones the backend
synthesizes from it. A large struct move becomes a `memcpy` call, `f32::powf`
becomes a `powf` call, `f128` arithmetic becomes a `compiler_builtins` call —
none of which appear as MIR terminators. Those symbols are assumed to be the
platform's. A program that defines one of them itself, in Rust, and
allocates there (`#[no_mangle] extern "C" fn powf`) defeats the analysis, and
it does so whether or not any intrinsic is involved. See
[ADR 0005](../adr/0005-intrinsic-leaf-classification.md) for the measurement
and the alternative.

**`cargo no-alloc -- test` is not usable for realistic code.** Cargo ignores
`[profile.*] panic` for the `test` profile — it always builds tests under
`panic=unwind`, regardless of what the manifest sets — so every `Assert`
terminator rejects under `-- test` no matter how the profile is configured.
`-- build` is the supported mode for meaningful results; `-- test` is useful
only for trivial, assertion-free code and is not a supported path for
checking real workloads.

Cross-crate generic roots are checked at downstream monomorphization sites.
Per-rustc fragments are merged deterministically, and unmatched or non-function
`--root` specifications are reported rather than silently disappearing. A
`#[no_alloc]` marker on a non-generic function in a non-workspace dependency
is a different case — see [`--all-crates`](usage.md#instrumentation-scope). `async fn` roots (marked or
`--root`) are a selection error, since only the future constructor has MIR.

**Further limitations, not closed by the checker:**

- The guarantee is about the `__rust_alloc`-family entry points (the
  allocator, deallocator, reallocator, and zeroed-allocator codegen
  attributes). A direct method call on a user `#[global_allocator]` static's
  `GlobalAlloc` impl, or on an allocator crate's API, is ordinary code and is
  followed like any other call: it is a violation only if that code reaches
  the entry points.
- The checked build is not the shipped build. It uses the pinned nightly's
  standard library and compiler, sets `--cfg no_alloc_check`, and the marker
  adds `rustc_no_mir_inline` to roots. Code that differs under
  `cfg(no_alloc_check)` or under another toolchain is checked as built here.
- `--root` on an unmarked generic function can still be MIR-inlined into all
  its callers and reported `NotInstantiated`, which counts as success. The
  marker prevents this for marked roots.
- A callee with explicit linkage (`#[linkage = "weak"]` and similar) is
  rejected: the linker may bind the symbol to a different definition than the
  MIR checked. A field projection into a type with an extern-type tail is
  rejected too, because codegen emits a panic call for it that no MIR edge
  names ([ADR 0003](../adr/0003-reject-unresolved-edges.md)).

See [`docs/design.md`](design.md) and the [`adr/`](../adr) directory for the
design rationale and operational details.
