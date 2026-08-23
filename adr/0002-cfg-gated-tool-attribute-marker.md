# ADR 0002: cfg-gated tool-attribute marker

## Status

Accepted.

## Context

Roots need to be marked somehow. The marker must be usable on stable Rust
(most users' normal build), must have zero effect on normal builds (no
codegen impact, no leftover attribute for other tools to trip over), and
must still be legible to the checker driver, which runs on a pinned nightly
and can use unstable features freely.

## Decision

`#[no_alloc::no_alloc]` is a proc-macro attribute that expands to the
annotated item unchanged, with one attribute prepended:

```rust
#[cfg_attr(no_alloc_check, no_alloc_tool::root)]
```

- On a normal build, `no_alloc_check` is unset. `cfg_attr` is evaluated in
  the *user's* crate (not the proc macro's host compilation), so it simply
  evaluates away — no nightly features, no residue, works on stable.
- The checker driver passes `--cfg no_alloc_check` plus
  `-Zcrate-attr=feature(register_tool)` and
  `-Zcrate-attr=register_tool(no_alloc_tool)`, so under the checker the
  surviving `#[no_alloc_tool::root]` is a valid (registered) tool attribute,
  inert to codegen, readable from `TyCtxt` via `codegen_fn_attrs` / HIR
  attribute queries.

The tool namespace is `no_alloc_tool`, deliberately distinct from the crate
name `no_alloc` — a tool namespace matching the crate name would make
`no_alloc::no_alloc` ambiguous between the tool-attribute namespace and the
proc-macro path.

## Consequences

- Users on stable Rust pay no cost: no unstable features, no marker residue
  in metadata, no codegen difference. This was a hard requirement, not a
  nice-to-have — the tool must not force nightly on people who only build
  normally.
- Users must silence `unexpected_cfgs` for `no_alloc_check` via
  `[lints.rust] unexpected_cfgs = { check-cfg = [...] }`. This is the one
  piece of friction the design imposes; documented in the README.
- **Resolved at M2**: inert tool attributes *do* survive into cross-crate
  metadata at `nightly-2026-08-01`. `rustc_metadata`'s encoder
  (`should_encode_attrs` / `analyze_attr` in `rmeta/encoder.rs`) encodes
  `DefKind::Fn` attributes by default; the only attributes it drops are
  lint attrs (`warn`/`allow`/etc., not applicable here) and unexported doc
  comments. An unrecognized-shape attribute like `#[no_alloc_tool::root]`
  (`hir::Attribute::Unparsed`, not `Parsed`) falls through to
  `should_encode = true`. Confirmed empirically with a two-crate fixture:
  `TyCtxt::get_attrs_by_path` found the attribute on a foreign `DefId` via
  its `attrs_for_def` `separate_provide_extern` provider. See
  `docs/design.md` for the fixture and log output.
- Consequence: the sidecar root index (see `docs/design.md`) is a **belt**,
  not the mechanism — direct cross-crate attribute reading already works.
  It is kept anyway per the original design (fallback for e.g. non-`Fn`
  targets or future attribute-encoding changes), but root collection is not
  blocked on it.
- `NO_ALLOC_ROOTS` env var and the sidecar index both exist as fallbacks
  independent of whether the attribute plumbing works, so root-collection
  work is not blocked on this ADR's mechanism being fully proven.

## Alternatives considered

**Unconditional unstable tool attribute** (`#[no_alloc_tool::root]` directly,
no macro): rejected, forces `#![feature(register_tool)]` and nightly on every
build, including normal ones.

**A marker trait or const**: rejected, has real codegen presence (a symbol,
a vtable entry) even when unused, and doesn't compose with `fn` items the
way an attribute does.

**Doc-comment or naming convention markers** (e.g. requiring a `# no_alloc`
doc line or a `no_alloc_` prefix): rejected, stringly-typed, no compiler
enforcement that the marker is actually attached to what the user thinks,
and indistinguishable from coincidental text.
