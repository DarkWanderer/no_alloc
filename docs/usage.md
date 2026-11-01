# Usage

## Marker attribute

The macro rejects async functions, trait methods without bodies, and
non-function items because their executable body cannot yet be selected
soundly.

## Command line

Run the checker with Cargo arguments after `--`:

```text
cargo no-alloc [--all-crates] [--build-std] [--strict]
               [--warn-only] [--root PATH]... -- [build|test] [CARGO_ARGS...]
```

`build` is the default. `check` is rejected because it does not produce the
monomorphized graph. `test` checks the code as compiled for tests (always
`panic=unwind`), and cannot be combined with `--strict` — see
[Guarantee and limitations](guarantees.md). The checker owns
its target and target directory, so user-supplied `--target` and
`--target-dir` are rejected. `--root` selects an unannotated function by
canonical path across the complete build; it accepts a comma-separated list
and may be repeated (`--root a,b --root c`). A malformed command line, such
as `--root a,,b`, is an argument error and writes no report.

## Instrumentation scope

By default only workspace members are instrumented (`RUSTC_WORKSPACE_WRAPPER`).
A `#[no_alloc]` marker on a non-generic function in a registry or path
dependency outside the workspace is therefore silently never analyzed — the
function never appears in the report at all, which is different from an
unmatched `--root` (reported as a selection error). A marked *generic*
function is checked wherever a workspace crate's mono graph reaches an
instantiation, including one a dependency already emitted and the workspace
crate links to under share-generics
([ADR 0004](../adr/0004-cross-crate-roots-and-report-aggregation.md)). Pass
`--all-crates` to instrument every crate in the build (`RUSTC_WRAPPER`)
instead.

## Cargo configuration

Unless `RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS` is set, the checked build uses
the `build.rustflags` / `target.<host>.rustflags` from your Cargo config, as a
plain `cargo build` would; `target.'cfg(...)'.rustflags` is not supported and
the run is refused (put those flags in `RUSTFLAGS`). A `rustc-wrapper` or
`rustc-workspace-wrapper` in Cargo config (or its `CARGO_BUILD_*` environment
form) is ignored for the run with a warning, because the driver must be the
only wrapper; the `RUSTC_WRAPPER` and `RUSTC_WORKSPACE_WRAPPER` environment
variables are refused. The checker compiles with the `rustc` found on `PATH`,
so a configured `build.rustc` is refused (a substitute compiler could add
`--cfg` flags the checked build would not see), as are the `RUSTC` and
`CARGO_BUILD_RUSTC` variables. A configured
`build.target` (`[build] target`, `CARGO_BUILD_TARGET`, `--config`) is
refused, since the checker always builds for the host; so is a workspace
member's `default-target` / `forced-target` (`per-package-target`). Keys starting
`NO_ALLOC_` in Cargo's `[env]` table are refused.

A non-UTF-8 `RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS` is refused. These
configuration refusals (rustc override, `RUSTC_WRAPPER`, ambient
immediate-abort without `--build-std`, non-UTF-8 flags) persist a failing
`report.json`; operational errors and argument errors do not.

## Report

The existing `NO_ALLOC_ROOTS`, `NO_ALLOC_WARN_ONLY`, and `NO_ALLOC_LOG`
environment interfaces remain supported. The final deterministic report is
written to `target/no-alloc/report.json`. Its `warnings` list (omitted when
empty) holds conditions that leave the exit code at 0 but that a reader of the
file must not mistake for a verified pass; today that is a run that checked
zero roots, including one where every root was `not_instantiated`. Each checked root carries a versioned
`environment` block with its typed panic strategy, optimization and MIR
optimization levels, target triple, rustc version, the
`--all-crates`/`--build-std` settings, and `panic_paths` — `"out_of_scope"`
for the default mode, `"checked"` under `--strict`
([ADR 0008](../adr/0008-panic-paths-out-of-scope-by-default.md)). A `Pass`
means something different in each, so read the two together. These values matter to what was proven,
and recording them per root preserves the real configuration when one Cargo
build contains both host and target artifacts. The report-level
`panic_strategy` remains for compatibility and is omitted when those artifacts
disagree.

## Clean rebuild on every run

Every invocation runs `cargo clean` on the checker's target first, so
**every `cargo no-alloc` run is a from-scratch rebuild**, never an
incremental one. This is deliberate — it guarantees that a checker
configuration change can never be hidden by Cargo's build cache (see
[`docs/design.md`](design.md)) — but it means every run pays full
compile time, not just the cost of what changed.

## Strict mode

By default panic paths are out of scope: the guarantee is "no allocation on
any non-panicking path" ([`guarantees.md`](guarantees.md)). `--strict` checks
them too. It rebuilds the crate and the standard library with
`-Zunstable-options -Cpanic=immediate-abort` (implying `--build-std`, and
incompatible with `-- test`, `--tests` and `--all-targets`, which rustc will
not build under an abort strategy), so every panic lowers to a bare `abort()`
the traversal walks to — see
[ADR 0006](../adr/0006-immediate-abort-checking-mode.md) and
[ADR 0008](../adr/0008-panic-paths-out-of-scope-by-default.md). The sysroot
rebuild makes each run take minutes. `--immediate-abort`, the previous name
for this mode, has been removed and is an unknown option.

## Examples

See [`examples/basic`](../examples/basic) for a runnable example, and
[`examples/iterators`](../examples/iterators) for the 35 iterator patterns
behind `docs/iterators.md`.
