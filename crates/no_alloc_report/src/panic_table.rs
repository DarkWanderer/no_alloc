//! The panic entry-point table: the standard-library functions at which the
//! default checking mode stops walking, because everything past them is a
//! panic and panics are outside the default guarantee (ADR 0008).
//!
//! The default guarantee is "no allocation on any non-panicking path". A
//! panic path is one that ends in a panic, so the walk can stop at the
//! function that *starts* one. Allocation on the way there (the `Box` that
//! `begin_panic` builds for its payload, the message `#[panic_handler]`
//! formats and prints) is allocation on a panicking path, and out of scope;
//! so is user code the panic machinery calls, such as a `no_std` crate's
//! `#[panic_handler]` or a `std::panic::set_hook` hook. What is *not* out of
//! scope is unwinding cleanup: the traversal visits every basic block of
//! every body it walks, so the cleanup blocks and drop glue of the root and
//! of every walked callee are still checked. And `catch_unwind` is not in
//! the intrinsic table, so a root that catches a panic still rejects.
//!
//! Each entry asserts one fact about the pinned sysroot
//! (`nightly-2026-10-31`), checked by reading its source: **every path
//! through this function ends in a panic.** Not "it is `#[cold]`", not "it
//! returns `!`" — `process::abort`, `process::exit` and
//! `alloc::alloc::handle_alloc_error` all diverge without being panics, and
//! a non-panicking path that allocates and then aborts is exactly what the
//! default guarantee still covers. So the table is an **allowlist**: a
//! diverging function that is not named here is walked or, without MIR,
//! rejected like any other callee (ADR 0003), and `tests/ui/panic_table_unknown_rejects`
//! pins that.
//!
//! Deliberately absent:
//! - `std::process::abort`, `std::process::exit`, `core::intrinsics::abort_immediate`,
//!   `alloc::alloc::handle_alloc_error` — diverging, not panics. The
//!   allocation-error handler is the clearest case: it is the path an
//!   allocation *failure* takes, so excusing it would excuse the allocator.
//! - `alloc::raw_vec::handle_error` — it panics for a capacity overflow but
//!   calls `handle_alloc_error` for an allocation failure, so not every path
//!   through it is a panic. `alloc::raw_vec::capacity_overflow`, which it
//!   calls for the first case, is in the table.
//! - `core::panicking::const_panic_fmt` — a const-eval replacement for
//!   `panic_fmt`, never called at runtime; its runtime body for a formatted
//!   message is `unreachable_unchecked`, which is not a panic.
//! - `std::panic::resume_unwind` and `std::panicking::rust_panic_without_hook`
//!   — they re-raise a payload someone already caught, which needs
//!   `catch_unwind` (rejected) to have happened first; not worth a soundness
//!   claim.
//! - Panic helpers nested inside other items (the `assert_failed` functions
//!   inside `Vec::insert`/`remove`/`swap_remove`/`split_off`, the
//!   `len_mismatch_fail` inside `copy_from_slice`, `const_panic!`'s
//!   `do_panic`). Their paths go through `impl` blocks or other functions;
//!   see "Matching" below. A caller reaching one is walked into it, which is
//!   precise or rejecting, never unsound.
//! - `std::thread::local::panic_access_error`, `std::sync::lazy_lock::panic_poisoned`
//!   and `core::num::from_ascii_bytes_radix_panic` — panic-only, but the code
//!   reaching the first two (TLS, `Once`) rejects for other reasons, and none
//!   has been needed. Adding one is a claim to be checked, not a family to
//!   complete.
//!
//! **Matching.** The key is the function's *defining* path, crate name
//! first: `core::option::unwrap_failed`, never `std::option::unwrap_failed`.
//! `tcx.def_path_str` is not that string — it prints the shortest visible
//! path (the `std::` re-export, under the `std` prelude), omits the crate
//! for a local item, and depends on printing context — so
//! `no_alloc_analysis` builds the key from the `DefPath` directly, accepts
//! only plain module/function components (an `impl` or closure segment
//! makes the item ineligible), and admits a crate as `core`/`alloc`/`std`
//! only if it is the crate that defines that library's anchor lang item. A
//! user crate with a module called `core::panicking` cannot match
//! (`tests/ui/panic_table_user_path`). The residual: in a graph with no real
//! `std`/`alloc`, a crate named `std` or `alloc` (or a `#![no_core]` crate
//! named `core`) that defines the anchor lang item itself would be accepted.
//! That needs `#![feature(lang_items)]` and a crate name Cargo normally
//! refuses, so it is a deliberate act, not an accident. A key spelled any
//! other way here simply never matches, which fails closed.
//!
//! The table is consulted only by the default checking mode. Under
//! `--strict` it is ignored and these functions are walked (ADR 0006).
//!
//! Like the intrinsic table it lives in this crate because it is a pure
//! function of a string, unit-testable on stable.

/// Whether `defining_path` names a standard-library function every path
/// through which ends in a panic. `defining_path` is the crate-qualified
/// defining path described in the module docs, e.g.
/// `"core::option::unwrap_failed"`.
pub fn is_panic_entry_point(defining_path: &str) -> bool {
    is_core_panicking(defining_path)
        || is_panic_const(defining_path)
        || is_failure_helper(defining_path)
        || is_integer_panic(defining_path)
        || is_std_panic(defining_path)
}

/// `core::panicking`: the two functions every panic goes through
/// (`panic_fmt`, `panic_nounwind_fmt`), the wrappers the `panic!` family
/// expands to, the lang items codegen calls for a failing `Assert`
/// (`panic_bounds_check`, `panic_misaligned_pointer_dereference`, ...) and
/// for an unwind out of a `nounwind` function (`panic_cannot_unwind`,
/// `panic_in_cleanup`), and the `assert_eq!`/`assert_matches!` failure
/// paths. Each is `-> !` and calls only formatting and another entry here.
fn is_core_panicking(path: &str) -> bool {
    matches!(
        path,
        "core::panicking::panic_fmt"
            | "core::panicking::panic_nounwind_fmt"
            | "core::panicking::panic"
            | "core::panicking::panic_nounwind"
            | "core::panicking::panic_nounwind_nobacktrace"
            | "core::panicking::panic_display"
            | "core::panicking::panic_str_2015"
            | "core::panicking::unreachable_display"
            | "core::panicking::panic_bounds_check"
            | "core::panicking::panic_misaligned_pointer_dereference"
            | "core::panicking::panic_null_pointer_dereference"
            | "core::panicking::panic_null_reference_constructed"
            | "core::panicking::panic_invalid_enum_construction"
            | "core::panicking::panic_cannot_unwind"
            | "core::panicking::panic_in_cleanup"
            | "core::panicking::assert_failed"
            | "core::panicking::assert_matches_failed"
            | "core::panicking::assert_failed_inner"
    )
}

/// `core::panicking::panic_const`: the lang items a failing overflow,
/// division or coroutine-state `Assert` lowers to, one per message, each
/// `panic_fmt(Arguments::from_str(MESSAGE))`. Listed by name rather than by
/// prefix so a new one is classified on purpose.
fn is_panic_const(path: &str) -> bool {
    let Some(name) = path.strip_prefix("core::panicking::panic_const::") else {
        return false;
    };
    matches!(
        name,
        "panic_const_add_overflow"
            | "panic_const_sub_overflow"
            | "panic_const_mul_overflow"
            | "panic_const_div_overflow"
            | "panic_const_rem_overflow"
            | "panic_const_neg_overflow"
            | "panic_const_shr_overflow"
            | "panic_const_shl_overflow"
            | "panic_const_div_by_zero"
            | "panic_const_rem_by_zero"
            | "panic_const_coroutine_resumed"
            | "panic_const_async_fn_resumed"
            | "panic_const_async_gen_fn_resumed"
            | "panic_const_gen_fn_none"
            | "panic_const_coroutine_resumed_panic"
            | "panic_const_async_fn_resumed_panic"
            | "panic_const_async_gen_fn_resumed_panic"
            | "panic_const_gen_fn_none_panic"
            | "panic_const_coroutine_resumed_drop"
            | "panic_const_async_fn_resumed_drop"
            | "panic_const_async_gen_fn_resumed_drop"
            | "panic_const_gen_fn_none_drop"
    )
}

/// The out-of-line failure paths of common library operations, split out of
/// their callers so the hot path stays small: `unwrap`/`expect` on `Option`
/// and `Result`, range indexing on slices and `str`, a conflicting
/// `RefCell` borrow, a poisoned `LazyCell`, `Vec`/`RawVec` capacity
/// overflow, and the sort's report of an inconsistent comparison function.
/// Each one's body is a `panic!` (in `slice_index_fail` and the
/// `RefCell` pair through `const_panic!`, whose runtime arm is a `panic!`;
/// in `str::slice_error_fail` through `const_eval_select`, both of whose
/// arms, `slice_error_fail_ct` and `slice_error_fail_rt`, end every branch
/// in a `panic!`).
fn is_failure_helper(path: &str) -> bool {
    matches!(
        path,
        "core::option::unwrap_failed"
            | "core::option::expect_failed"
            | "core::result::unwrap_failed"
            | "core::slice::index::slice_index_fail"
            | "core::str::slice_error_fail"
            | "core::str::slice_error_fail_ct"
            | "core::str::slice_error_fail_rt"
            | "core::cell::panic_already_borrowed"
            | "core::cell::panic_already_mutably_borrowed"
            | "core::cell::lazy::panic_poisoned"
            | "core::slice::sort::shared::smallsort::panic_on_ord_violation"
            | "alloc::raw_vec::capacity_overflow"
    )
}

/// `core::num::imp`'s shared panic paths for integer operations: the
/// `strict_*` arithmetic overflow panics (`overflow_panic`), and the
/// non-positive and negative argument panics of `ilog*` and `isqrt`. Each body
/// is a single `panic!` with a literal message. `ilog2` is reached from
/// `slice::sort_unstable`'s recursion limit, which is how this was found.
fn is_integer_panic(path: &str) -> bool {
    if let Some(name) = path.strip_prefix("core::num::imp::overflow_panic::") {
        return matches!(
            name,
            "add" | "sub" | "mul" | "rem" | "neg" | "shr" | "shl" | "pow" | "cast_integer"
        );
    }
    matches!(
        path,
        "core::num::imp::int_log10::panic_for_nonpositive_argument"
            | "core::num::imp::int_sqrt::panic_for_negative_argument"
    )
}

/// `std`'s own panic entry: `begin_panic` (the `panic!` of a non-string
/// payload, and the `begin_panic` lang item) and `std::panic::panic_any`,
/// which only calls it. `begin_panic` boxes the payload, which is the
/// allocation-on-a-panicking-path this mode excludes by definition.
fn is_std_panic(path: &str) -> bool {
    matches!(
        path,
        "std::panicking::begin_panic" | "std::panic::panic_any"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_points_explicit_panics_reach_are_in_the_table() {
        for path in [
            "core::panicking::panic_fmt",
            "core::panicking::panic",
            "core::panicking::panic_nounwind_fmt",
            "core::panicking::panic_bounds_check",
            "core::panicking::panic_const::panic_const_add_overflow",
            "core::panicking::panic_const::panic_const_div_by_zero",
            "core::option::unwrap_failed",
            "core::option::expect_failed",
            "core::result::unwrap_failed",
            "core::slice::index::slice_index_fail",
            "core::cell::panic_already_borrowed",
            "alloc::raw_vec::capacity_overflow",
            "std::panicking::begin_panic",
            "core::num::imp::int_log10::panic_for_nonpositive_argument",
            "core::num::imp::overflow_panic::add",
        ] {
            assert!(is_panic_entry_point(path), "{path}");
        }
    }

    /// Diverging is not panicking. Each of these ends a non-panicking path
    /// (or, for `handle_error`, can), so a root that allocates and then
    /// reaches one must still be judged on that allocation.
    #[test]
    fn diverging_functions_that_are_not_panics_are_absent() {
        for path in [
            "std::process::abort",
            "std::process::exit",
            "core::intrinsics::abort",
            "core::intrinsics::abort_immediate",
            "core::process::abort_immediate",
            "alloc::alloc::handle_alloc_error",
            "std::alloc::rust_oom",
            "alloc::raw_vec::handle_error",
            "core::hint::unreachable_unchecked",
            "core::panicking::const_panic_fmt",
            "std::panic::resume_unwind",
        ] {
            assert!(!is_panic_entry_point(path), "{path}");
        }
    }

    /// The key is the defining path. `def_path_str` prints the visible
    /// re-export instead (`std::option::unwrap_failed` under the `std`
    /// prelude, as `strict_host_panic_reject`'s chain still shows), and a
    /// local item with no crate prefix at all; neither spelling may match,
    /// so a caller that forgets to normalise fails closed.
    #[test]
    fn only_the_defining_crate_spelling_matches() {
        for path in [
            "std::option::unwrap_failed",
            "std::panicking::panic_fmt",
            "std::rt::panic_fmt",
            "std::rt::begin_panic",
            "panicking::panic_fmt",
            "option::unwrap_failed",
            "::core::panicking::panic_fmt",
            "crate::panicking::panic_fmt",
        ] {
            assert!(!is_panic_entry_point(path), "{path}");
        }
    }

    /// An allowlist matches whole names; nothing is a prefix family.
    #[test]
    fn near_misses_and_unknown_names_do_not_match() {
        for path in [
            "",
            "core::panicking",
            "core::panicking::panic_const",
            "core::panicking::panic_const::",
            "core::panicking::panic_const::panic_const_pow_overflow",
            "core::panicking::panic_fmt2",
            "core::panicking::panic_fmt::inner",
            "core::option::unwrap_failed_",
            "mycrate::panicking::panic_fmt",
            "core::panicking::panic_impl",
            "core::num::imp::overflow_panic::div",
            "core::num::imp::overflow_panic",
        ] {
            assert!(!is_panic_entry_point(path), "{path}");
        }
    }
}
