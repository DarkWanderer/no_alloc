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

For a `mem::swap` of a non-SSA type in a local root, the intrinsic's fallback
body is walked conservatively when a linked dependency built at the same
opt-level instantiates the same `mem::swap::<T>` (share-generics makes it
upstream-shared), even if that dependency lowered the swap in place
([ADR 0005](../adr/0005-intrinsic-leaf-classification.md)). On the pinned
sysroot the only edge in that body that can reject is its UB-check panic, so
this is a visible false rejection only where that panic is both in scope and
unresolvable: a host unit (build script) under `--strict`. In the default
mode the panic is out of scope, and on the target under `--strict` it ends
in `abort`, so the root passes either way.

**Panic paths are out of scope by default.** The default guarantee is *no
allocation on any non-panicking path*, under the build's own panic strategy
(`unwind` included) and recorded as `panic_paths: "out_of_scope"` on each
root's `environment` ([ADR 0008](../adr/0008-panic-paths-out-of-scope-by-default.md)).
Two things end the walk without being followed:

- a call to a standard-library panic entry point named in the audited panic
  table (`crates/no_alloc_report/src/panic_table.rs`: `panic_fmt`,
  `unwrap_failed`, `expect_failed`, `slice_index_fail`, the `panic_const_*`
  overflow lang items, `begin_panic`, ...), matched by defining path;
- an `Assert` terminator (the implicit check for array bounds, arithmetic
  overflow, division by zero), whatever the panic strategy.

So `.unwrap()`, `.expect()`, indexing, `panic!()` and iterator chains pass
when nothing else in them allocates or rejects. This is a scope exclusion, not a proof:
with `std`, the `#[panic_handler]` formats and prints, `begin_panic` boxes
its payload, and none of that is examined. What *is* still examined on a
panicking path: everything the walked code does before calling the entry
point (a `String` built for a panic message is a violation), and the
unwinding cleanup of every walked body, the root's and every callee's —
the traversal visits every basic block of each body it walks, cleanup
blocks and their drop glue included. Catching a panic needs `catch_unwind`,
which rejects. Also excluded, because it only runs inside a panic: user
code that the panic machinery calls, such as a `no_std` crate's own
`#[panic_handler]` or a hook installed with `std::panic::set_hook`. A diverging function that is not
a panic (`process::abort`, `process::exit`, `handle_alloc_error`) is not in
the table and is walked or rejected like any other call.

`UnwindResume` and `UnwindTerminate` have no MIR callee to follow, so the
traversal treats them as terminal control flow rather than calls — but each
lowers to a real call during codegen (a foreign `_Unwind_Resume`, or a
nounwind panic-handler call) that the traversal never walks. `Unreachable`
codegens to a bare LLVM `unreachable` and contains no code at all. These are
out of scope in both modes ([ADR 0003](../adr/0003-reject-unresolved-edges.md)).

The carve-out covers a panic on one *branch* of an executable path, not a
construct whose *only* lowering is a panic. A failing validity assertion
(`mem::zeroed::<&T>()`, undefined behaviour on every call that reaches it)
and a field projection into an extern-type tail therefore reject in every
mode, by design — see
[ADR 0008](../adr/0008-panic-paths-out-of-scope-by-default.md#unconditional-panics-are-not-carved-out-amended-2026-11-01).

**`--strict` checks panic paths too.** It compiles the crate and the standard
library with `-Cpanic=immediate-abort`, where every panic — including the
panic lang item a failing `Assert` calls — lowers to a bare `abort()` with no
handler call. The panic table is not consulted: the traversal walks panic
paths, `Assert`'s included, as ordinary edges and finds
`core::intrinsics::abort` at the end of them, so in that mode there is no
panic-path carve-out at all (`panic_paths: "checked"`). It costs a full
sysroot rebuild per run, and it cannot be combined with `-- test`. See
[ADR 0006](../adr/0006-immediate-abort-checking-mode.md). Its verdict is
about the build the checker made: a crate shipped with `panic = "unwind"` and
checked with `--strict` still has an allocating panic runtime in its shipped
binary.

Measured over the 35 iterator patterns in `examples/iterators`, the default
mode passes 33 under both `panic = "unwind"` and `panic = "abort"`, and
`--strict` passes the same 33; the other two are a real allocation and a real
function pointer — see [`iterators.md`](iterators.md).

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

**`cargo no-alloc -- test` checks the code as compiled for tests.** Cargo
builds tests under `panic=unwind` whatever the profile says, which the
default mode handles like any other build. `--strict` cannot be combined
with `-- test` (or `--tests`/`--all-targets`), because rustc refuses to
build a libtest harness under an abort panic strategy.

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
  rejected too, in every mode: codegen lowers it to an unconditional panic
  call that no MIR edge names ([ADR 0003](../adr/0003-reject-unresolved-edges.md),
  [ADR 0008](../adr/0008-panic-paths-out-of-scope-by-default.md#unconditional-panics-are-not-carved-out-amended-2026-11-01)).

See [`design.md`](design.md) and the [`adr/`](../adr) directory for the
design rationale and operational details.
