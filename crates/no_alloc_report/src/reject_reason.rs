use std::fmt;

/// Why the traversal rejected a root instead of proving or refuting it.
///
/// The report schema stores the rendered text (`Verdict::Rejected::reason`),
/// and UI snapshots pin that text, so the `Display` output is part of the
/// tool's observable contract. Keeping every reason as a variant makes the
/// set of rejection causes enumerable in one place, rather than scattered
/// as string literals through the traversal (ADR 0003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    /// An intrinsic whose name the audited table (ADR 0005) does not list.
    IntrinsicNotInTable {
        name: String,
    },
    /// `InstanceKind::Intrinsic` with no `IntrinsicDef` behind it.
    UnidentifiableIntrinsic,
    /// `assert_inhabited` and friends, for an instantiation whose
    /// requirement fails and therefore compiles to a panic call.
    ValidityAssertionFails {
        intrinsic: String,
        ty: String,
    },
    /// A field projection into a type with an extern-type tail.
    ExternTailProjection,
    /// Foreign, bodiless, intrinsic or virtual callee: no MIR to walk.
    NoMirBody,
    /// `#[linkage]`/import linkage on the callee.
    ExplicitLinkage,
    /// `Rvalue::ThreadLocalRef` or `ShimKind::ThreadLocal`.
    ThreadLocalAccess,
    CoroutineClosureShim,
    AsyncDropGlue,
    /// `Instance::try_resolve` found nothing for a `FnDef` callee.
    CalleeUnresolved {
        path: String,
    },
    /// A `FnDef` callee whose generic args still contain bound variables.
    CalleeHasBoundVars {
        path: String,
    },
    FunctionPointerCallee,
    DropThroughVtable,
    InlineAsm,
    AssertUnderUnwind,
    PreLoweringTerminator,
    /// The panic lang item an `Assert` lowers to is missing.
    LangItemUndefined {
        name: String,
    },
}

impl fmt::Display for RejectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IntrinsicNotInTable { name } => write!(
                f,
                "intrinsic `{name}` is not in the non-allocating intrinsic table"
            ),
            Self::UnidentifiableIntrinsic => f.write_str("unidentifiable intrinsic callee"),
            Self::ValidityAssertionFails { intrinsic, ty } => write!(
                f,
                "`{intrinsic}::<{ty}>` does not hold, so this call compiles to a panic"
            ),
            Self::ExternTailProjection => f.write_str(
                "projection into an extern-type tail: codegen computes its size with a `panic_nounwind` call",
            ),
            Self::NoMirBody => f.write_str("no statically available MIR body for this callee"),
            Self::ExplicitLinkage => f.write_str(
                "callee has explicit linkage; the linker may bind it to a different definition",
            ),
            Self::ThreadLocalAccess => f.write_str(
                "thread-local access is not modeled (may call into the TLS runtime on some targets)",
            ),
            Self::CoroutineClosureShim => f.write_str(
                "coroutine-closure construction shim is not yet audited by this tool",
            ),
            Self::AsyncDropGlue => f.write_str("async drop glue is not yet audited by this tool"),
            Self::CalleeUnresolved { path } => write!(
                f,
                "callee `{path}` could not be resolved to a concrete implementation"
            ),
            Self::CalleeHasBoundVars { path } => {
                write!(f, "callee `{path}` is still generic over bound variables")
            }
            Self::FunctionPointerCallee => {
                f.write_str("callee is a function pointer, not a statically resolvable body")
            }
            Self::DropThroughVtable => f.write_str(
                "drop through a vtable: the concrete destructor is not statically known",
            ),
            Self::InlineAsm => f.write_str("inline assembly can call or do anything opaquely"),
            Self::AssertUnderUnwind => {
                f.write_str("assertion may invoke the panic handler when panic=unwind")
            }
            Self::PreLoweringTerminator => {
                f.write_str("unexpected pre-lowering control-flow terminator")
            }
            Self::LangItemUndefined { name } => write!(
                f,
                "assertion panics through lang item `{name}`, which is not defined"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(name: &str) -> String {
        name.to_string()
    }

    /// The rendered text lands in `report.json` and UI snapshots; several of
    /// these reasons have no UI case at all, so this is their only pin.
    #[test]
    fn display_text_is_stable() {
        let cases = [
            (
                RejectReason::IntrinsicNotInTable { name: s("va_arg") },
                "intrinsic `va_arg` is not in the non-allocating intrinsic table",
            ),
            (
                RejectReason::UnidentifiableIntrinsic,
                "unidentifiable intrinsic callee",
            ),
            (
                RejectReason::ValidityAssertionFails {
                    intrinsic: s("assert_inhabited"),
                    ty: s("!"),
                },
                "`assert_inhabited::<!>` does not hold, so this call compiles to a panic",
            ),
            (
                RejectReason::ExternTailProjection,
                "projection into an extern-type tail: codegen computes its size with a `panic_nounwind` call",
            ),
            (
                RejectReason::NoMirBody,
                "no statically available MIR body for this callee",
            ),
            (
                RejectReason::ExplicitLinkage,
                "callee has explicit linkage; the linker may bind it to a different definition",
            ),
            (
                RejectReason::ThreadLocalAccess,
                "thread-local access is not modeled (may call into the TLS runtime on some targets)",
            ),
            (
                RejectReason::CoroutineClosureShim,
                "coroutine-closure construction shim is not yet audited by this tool",
            ),
            (
                RejectReason::AsyncDropGlue,
                "async drop glue is not yet audited by this tool",
            ),
            (
                RejectReason::CalleeUnresolved { path: s("a::f") },
                "callee `a::f` could not be resolved to a concrete implementation",
            ),
            (
                RejectReason::CalleeHasBoundVars { path: s("a::f") },
                "callee `a::f` is still generic over bound variables",
            ),
            (
                RejectReason::FunctionPointerCallee,
                "callee is a function pointer, not a statically resolvable body",
            ),
            (
                RejectReason::DropThroughVtable,
                "drop through a vtable: the concrete destructor is not statically known",
            ),
            (
                RejectReason::InlineAsm,
                "inline assembly can call or do anything opaquely",
            ),
            (
                RejectReason::AssertUnderUnwind,
                "assertion may invoke the panic handler when panic=unwind",
            ),
            (
                RejectReason::PreLoweringTerminator,
                "unexpected pre-lowering control-flow terminator",
            ),
            (
                RejectReason::LangItemUndefined { name: s("panic_bounds_check") },
                "assertion panics through lang item `panic_bounds_check`, which is not defined",
            ),
        ];
        for (reason, text) in cases {
            assert_eq!(reason.to_string(), text);
        }
    }
}
