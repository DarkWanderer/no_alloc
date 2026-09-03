# Security Policy

`no_alloc` is a pre-1.0 static analysis tool with a deliberately narrow,
documented soundness claim: for a marked root, resolved calls, tail calls,
and required drop glue on its *non-panicking* execution paths cannot reach
the global allocator. That scope exclusion is drawn by MIR terminator shape,
not by "does this path panic": it excludes `Unreachable`, `UnwindResume`,
and `UnwindTerminate` terminators, and an `Assert` terminator under a
non-unwinding panic strategy — see README's "Guarantee and limitations" and
[ADR 0003](adr/0003-reject-unresolved-edges.md). An *explicit* panic
(`panic!()`, `.unwrap()`, `.expect()`, ...) is a `Call` terminator, not one
of those — it is in scope, and rejects (fails the build) rather than being
excluded, since its callee is foreign/bodiless.

## What counts as a vulnerability here

**A known limitation, not a vulnerability:** the checker passing code whose
only allocation path runs through something the guarantee explicitly
excludes — a panic handler, an `Unreachable`/`UnwindResume`/`UnwindTerminate`
terminator, or an `Assert` allowed under a non-unwinding panic strategy.
These are documented scope exclusions, not bugs. Please don't file a report
for one; open a normal issue if the documentation itself is unclear about
where the scope ends.

**A real vulnerability:** the checker passing a root that reaches the
allocator through a path the guarantee claims to cover — e.g. via a resolved
call, tail call, or drop glue on a non-panicking path, with no unresolved
edge, virtual dispatch, function pointer, or foreign call involved. That
would mean the traversal itself is unsound, contradicting "reject, don't
assume" ([ADR 0003](adr/0003-reject-unresolved-edges.md)). This is the class
of bug we want reported privately rather than filed as a public issue.

## Reporting

Please report suspected soundness bugs (as scoped above) through GitHub
private security advisories rather than a public issue:

<https://github.com/DarkWanderer/no_alloc/security/advisories/new>

Include the smallest reproducing crate you can manage — ideally in the same
shape as the fixtures under `tests/ui/<case>/` — along with the `cargo
no-alloc` invocation and the report it produced.

There is no fixed response-time SLA at this stage of the project; expect an
acknowledgment, not a guarantee.

## Supported versions

Pre-1.0: only the latest published release is supported. There is no
backport policy yet.
