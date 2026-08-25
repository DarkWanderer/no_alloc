//! The non-allocating intrinsic table — the second leaf set, alongside the
//! allocator predicate in `no_alloc_analysis::leaf`.
//!
//! A `#[rustc_intrinsic]` callee is *not* an unresolved edge. `try_resolve`
//! resolved it exactly, to `InstanceKind::Intrinsic`; what it lacks is a MIR
//! body, because the body lives in the compiler's backend rather than in
//! Rust source. Treating that as "unresolved, therefore reject" (which is
//! what this traversal did before) is the wrong reading of ADR 0003: there
//! is nothing here to be uncertain about, and the rejection is what made
//! *every* iterator over a slice unverifiable — a panic path compiled under
//! `panic = "immediate-abort"` bottoms out in `intrinsics::abort`, and the
//! optimizer hints around `unlikely` bottom out in `intrinsics::cold_path`.
//! See ADR 0005.
//!
//! So intrinsics are classified, not assumed, and the classification is an
//! allowlist: an intrinsic named here lowers to machine instructions, an
//! LLVM intrinsic, or a `compiler_builtins`/libm/libc symbol (`memcpy`,
//! `memcmp`, `powf`, `__addtf3`, ...) — never to a call into Rust code that
//! the traversal could have walked instead. Anything not named here still
//! rejects, so a new intrinsic in a future toolchain is rejected until
//! someone has looked at it — the same direction of failure ADR 0003 asks
//! for everywhere else.
//!
//! One caveat, and it is the whole tool's rather than this table's: where
//! that lowering emits a *named* symbol, a program can define that symbol
//! itself — `#[no_mangle] extern "C" fn powf` — and the analysis will not
//! see it, because no such call exists in MIR to follow. Nothing about
//! intrinsics is special here: an ordinary struct move emits
//! `llvm.memcpy` and reaches the `memcpy` symbol with nothing in MIR at
//! all. ADR 0005 records the boundary; treating the intrinsic half of it as
//! a rejection would not close it, only make the tool inconsistent about
//! which half it reports.
//!
//! Deliberately *absent*, because each one runs a function its caller
//! supplied and the traversal cannot see what that function does:
//! `catch_unwind`, `const_eval_select`, `contract_check_requires`,
//! `contract_check_ensures`, `autodiff`, `offload`. Also absent:
//! `const_allocate`/`const_deallocate`/`const_make_global`, which model the
//! const-eval heap (they are erased before codegen, but a tool that says
//! "no allocator" should not be the one to wave through something spelled
//! `allocate`); the validity assertions `assert_inhabited`,
//! `assert_zero_valid` and `assert_mem_uninitialized_valid`, whose codegen
//! emits a `panic_nounwind` call for an instantiation that fails the
//! requirement (see [`is_layout_query`]); and the SIMD, GPU, `va_*`, and
//! `rustc_peek` families, which this table has not audited.
//!
//! Matching is on `IntrinsicDef::name`, which is the compiler's own dispatch
//! key for the intrinsic (`rustc_codegen_ssa` selects the lowering by this
//! exact symbol) — it is the intrinsic's identity, not a guess derived from
//! a linker symbol, and so is not the symbol-name matching ADR 0003 rules
//! out for allocator detection.
//!
//! The table lives in this crate rather than in `no_alloc_analysis` for the
//! same reason [`crate::parse_root_spec`] does: it is a pure function of a
//! name, so keeping it here makes it unit-testable on stable, with no
//! `TyCtxt` and no `rustc_private` build.

/// Whether an intrinsic's lowering provably contains no call into Rust code,
/// and so cannot reach the global allocator. `name` is the intrinsic's name
/// as the compiler knows it (`rustc_middle::ty::IntrinsicDef::name`), e.g.
/// `"abort"` — not a path and not a mangled symbol.
pub fn intrinsic_cannot_reach_allocator(name: &str) -> bool {
    is_control_or_hint(name)
        || is_layout_query(name)
        || is_memory_primitive(name)
        || is_integer_arithmetic(name)
        || is_float_arithmetic(name)
        || is_atomic(name)
}

/// Process control and optimizer hints. `abort` terminates the process
/// without running any Rust code (it is what a panic lowers to under
/// `panic = "immediate-abort"`); the rest are pure hints to LLVM that emit
/// no code of their own.
fn is_control_or_hint(name: &str) -> bool {
    matches!(
        name,
        "abort"
            | "assume"
            | "black_box"
            | "breakpoint"
            | "cold_path"
            | "is_val_statically_known"
            | "overflow_checks"
            | "prefetch_read_data"
            | "prefetch_read_instruction"
            | "prefetch_write_data"
            | "prefetch_write_instruction"
            | "select_unpredictable"
            | "ub_checks"
            | "unreachable"
    )
}

/// Type and layout queries. Every one of these folds to a constant, a
/// pointer to a `static`, or a vtable field read.
///
/// Not `assert_inhabited`, `assert_zero_valid` or
/// `assert_mem_uninitialized_valid`, which look like layout queries and are
/// not: for an instantiation that fails the requirement, codegen emits a
/// real call to the `panic_nounwind` lang item
/// (`rustc_codegen_ssa::mir::block::codegen_panic_intrinsic`) — a Rust
/// function this traversal never sees, since the call is synthesized after
/// MIR. Outside `-Cpanic=immediate-abort` that reaches the allocating panic
/// handler, so classifying them here would have been exactly the unsound
/// "assume safe" this table exists to avoid.
fn is_layout_query(name: &str) -> bool {
    matches!(
        name,
        "align_of"
            | "align_of_val"
            | "caller_location"
            | "discriminant_value"
            | "needs_drop"
            | "offset_of"
            | "ptr_metadata"
            | "size_of"
            | "size_of_val"
            | "type_id"
            | "type_name"
            | "variant_count"
            | "vtable_align"
            | "vtable_size"
    )
}

/// Loads, stores, and pointer arithmetic. The bulk-memory ones lower to
/// `memcpy`/`memmove`/`memset`/`memcmp`, which operate on memory the caller
/// already owns and never allocate.
fn is_memory_primitive(name: &str) -> bool {
    matches!(
        name,
        "aggregate_raw_ptr"
            | "arith_offset"
            | "compare_bytes"
            | "copy"
            | "copy_nonoverlapping"
            | "forget"
            | "nontemporal_store"
            | "offset"
            | "ptr_guaranteed_cmp"
            | "ptr_mask"
            | "ptr_offset_from"
            | "ptr_offset_from_unsigned"
            | "raw_eq"
            | "read_via_copy"
            | "slice_get_unchecked"
            | "transmute"
            | "transmute_unchecked"
            | "typed_swap_nonoverlapping"
            | "unaligned_volatile_load"
            | "unaligned_volatile_store"
            | "volatile_copy_memory"
            | "volatile_copy_nonoverlapping_memory"
            | "volatile_load"
            | "volatile_set_memory"
            | "volatile_store"
            | "write_bytes"
            | "write_via_move"
    )
}

/// Integer and bit arithmetic — single machine instructions, or the
/// `llvm.ctlz`/`llvm.ctpop`/... family.
fn is_integer_arithmetic(name: &str) -> bool {
    matches!(
        name,
        "add_with_overflow"
            | "bitreverse"
            | "bswap"
            | "carryless_mul"
            | "carrying_mul_add"
            | "ctlz"
            | "ctlz_nonzero"
            | "ctpop"
            | "cttz"
            | "cttz_nonzero"
            | "disjoint_bitor"
            | "exact_div"
            | "float_to_int_unchecked"
            | "mul_with_overflow"
            | "rotate_left"
            | "rotate_right"
            | "saturating_add"
            | "saturating_sub"
            | "sub_with_overflow"
            | "three_way_compare"
            | "unchecked_add"
            | "unchecked_div"
            | "unchecked_funnel_shl"
            | "unchecked_funnel_shr"
            | "unchecked_mul"
            | "unchecked_rem"
            | "unchecked_shl"
            | "unchecked_shr"
            | "unchecked_sub"
            | "wrapping_add"
            | "wrapping_mul"
            | "wrapping_sub"
    )
}

/// Floating-point arithmetic. Written as a base name plus the width suffix
/// the intrinsic set uses (`sqrtf16`/`sqrtf32`/`sqrtf64`/`sqrtf128`) rather
/// than four spellings of every operation. These lower to a machine
/// instruction, an `llvm.*` intrinsic, or — for `f16`/`f128` on targets
/// without hardware support — a `compiler_builtins` soft-float symbol.
///
/// Each base is matched against the exact spelling it has in the intrinsic
/// set, including whether an underscore separates it from the width, so
/// only the audited names match: `sqrtf32` yes, a future `sqrt_f32` no.
/// Normalizing the two spellings together would quietly admit a name nobody
/// has looked at, which is the opposite of what an allowlist is for.
fn is_float_arithmetic(name: &str) -> bool {
    if matches!(
        name,
        "fabs"
            | "fadd_algebraic"
            | "fadd_fast"
            | "fdiv_algebraic"
            | "fdiv_fast"
            | "fmul_algebraic"
            | "fmul_fast"
            | "frem_algebraic"
            | "frem_fast"
            | "fsub_algebraic"
            | "fsub_fast"
    ) {
        return true;
    }
    ["f16", "f32", "f64", "f128"].iter().any(|width| {
        name.strip_suffix(width).is_some_and(|base| {
            matches!(
                base,
                // `sqrtf32`
                "ceil"
                    | "copysign"
                    | "cos"
                    | "exp"
                    | "exp2"
                    | "floor"
                    | "fma"
                    | "fmuladd"
                    | "log"
                    | "log10"
                    | "log2"
                    | "maximum"
                    | "minimum"
                    | "pow"
                    | "powi"
                    | "round"
                    | "sin"
                    | "sqrt"
                    | "trunc"
                    // `round_ties_even_f32` — the underscore belongs to the
                    // name, so it is matched rather than trimmed away.
                    | "maximum_number_nsz_"
                    | "minimum_number_nsz_"
                    | "round_ties_even_"
            )
        })
    })
}

/// Atomic read-modify-write operations and fences. Each is one instruction
/// (or a `compiler_builtins` `__atomic_*` libcall on targets that lack the
/// instruction); the ordering is a const generic argument, not a callee.
fn is_atomic(name: &str) -> bool {
    matches!(
        name,
        "atomic_and"
            | "atomic_cxchg"
            | "atomic_cxchgweak"
            | "atomic_fence"
            | "atomic_load"
            | "atomic_max"
            | "atomic_min"
            | "atomic_nand"
            | "atomic_or"
            | "atomic_singlethreadfence"
            | "atomic_store"
            | "atomic_umax"
            | "atomic_umin"
            | "atomic_xadd"
            | "atomic_xchg"
            | "atomic_xor"
            | "atomic_xsub"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The point of the table is what it *excludes*: an intrinsic that runs
    /// a caller-supplied function can reach anything, including the
    /// allocator, and must keep rejecting. Guards against someone
    /// completing a family ("all the `const_*` ones", "everything in
    /// `core::intrinsics`") without noticing what they swept in.
    #[test]
    fn dispatching_intrinsics_are_not_in_the_table() {
        for name in [
            "autodiff",
            "catch_unwind",
            "const_allocate",
            "const_deallocate",
            "const_eval_select",
            "const_make_global",
            "contract_check_ensures",
            "contract_check_requires",
            "offload",
        ] {
            assert!(
                !intrinsic_cannot_reach_allocator(name),
                "`{name}` must not be classified as non-allocating"
            );
        }
    }

    /// These read like layout queries and are not: codegen turns a failing
    /// instantiation into a `panic_nounwind` call that the traversal cannot
    /// see, so classifying them would let an allocating panic handler
    /// through under plain `panic = "abort"`.
    #[test]
    fn validity_assertions_are_not_in_the_table() {
        for name in [
            "assert_inhabited",
            "assert_mem_uninitialized_valid",
            "assert_zero_valid",
        ] {
            assert!(
                !intrinsic_cannot_reach_allocator(name),
                "`{name}` compiles to a panic call and must keep rejecting"
            );
        }
    }

    /// `abort` is where every panic goes under `panic = "immediate-abort"`,
    /// and `cold_path` is what the hint around an overflow check lowers to.
    /// Between them they are the whole reason a slice iterator can be
    /// checked at all (ADR 0005) — a regression here silently takes the
    /// iterator subset back out.
    #[test]
    fn the_intrinsics_that_unblock_iterators_are_in_the_table() {
        assert!(intrinsic_cannot_reach_allocator("abort"));
        assert!(intrinsic_cannot_reach_allocator("cold_path"));
    }

    /// An unknown intrinsic rejects — the table is an allowlist, so a name
    /// nobody has classified must not fall through to "safe".
    #[test]
    fn unknown_names_are_not_in_the_table() {
        for name in ["", "definitely_not_an_intrinsic", "alloc", "sqrt"] {
            assert!(!intrinsic_cannot_reach_allocator(name), "{name}");
        }
    }

    #[test]
    fn float_width_suffixes_are_matched_per_width() {
        for name in ["sqrtf16", "sqrtf32", "sqrtf64", "sqrtf128", "powif32"] {
            assert!(intrinsic_cannot_reach_allocator(name), "{name}");
        }
        // The base name is only recognized with a width suffix attached,
        // and only for the four real widths.
        for name in ["sqrtf80", "round_ties_even", "readf32"] {
            assert!(!intrinsic_cannot_reach_allocator(name), "{name}");
        }
    }

    /// The table is an allowlist, so a name that merely *resembles* an
    /// audited one must not match. A future toolchain that spells an
    /// intrinsic `sqrt_f32` is introducing a lowering nobody here has
    /// looked at, and it should reject until someone does.
    #[test]
    fn near_miss_spellings_do_not_match() {
        for name in [
            "sqrt_f32",
            "sqrt__f32",
            "round_ties_evenf64",
            "round_ties_even__f64",
            "minimum_number_nszf32",
            "_sqrtf32",
        ] {
            assert!(!intrinsic_cannot_reach_allocator(name), "{name}");
        }
    }

    #[test]
    fn multi_word_base_names_keep_their_underscores() {
        assert!(intrinsic_cannot_reach_allocator("round_ties_even_f64"));
        assert!(intrinsic_cannot_reach_allocator("maximum_number_nsz_f32"));
        assert!(intrinsic_cannot_reach_allocator("minimum_number_nsz_f128"));
    }
}
