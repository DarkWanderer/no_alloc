//! Command-line parsing: the checker's own options, and the Cargo arguments
//! it forwards.

use crate::cargo_args::split_at_separator;
use anyhow::{bail, ensure, Context, Result};
use no_alloc_report::{parse_root_spec, PanicPaths};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::Path;

// A cargo subcommand is expected to answer `--help`/`-h`; kept as a plain
// string (rather than built from `Invocation`'s fields) so it reads as the
// user-facing contract, not an implementation detail that happens to leak.
pub(crate) const HELP_TEXT: &str = "\
cargo no-alloc [OPTIONS] -- [build|test] [CARGO_ARGS...]

Statically checks that #[no_alloc]-marked function instances cannot reach
the global allocator on any non-panicking path, then runs the underlying
Cargo command (`build` is used when [build|test] is omitted; `check` is
rejected because it never produces the monomorphized instance graph the
analysis runs on).

Options:
      --all-crates   Instrument every crate in the build (RUSTC_WRAPPER),
                     not just workspace members (RUSTC_WORKSPACE_WRAPPER)
      --build-std    Pass -Zbuild-std to Cargo
      --strict       Also check panic paths: compile everything, std
                     included, with -Cpanic=immediate-abort so a panic
                     lowers to a bare abort() the analysis can walk to.
                     Implies --build-std; cannot be combined with `test`.
                     See docs/guarantees.md
      --warn-only    Report findings on stderr without failing the build
      --root PATH    Additionally check an unannotated function by its
                     canonical path (repeatable; a value may also be a
                     comma-separated list)
  -h, --help         Print this help and exit
  -V, --version      Print version information and exit";

/// Cargo target selections that always pull in a libtest harness, which
/// cannot be built under `--strict` (see the `-- test` rejection).
///
/// Only these two: both include the unit-test targets of the lib and bins,
/// which use libtest whatever the manifest says. `--test`/`--bench` name one
/// target and `--benches` selects only bench targets — any of which may set
/// `harness = false` (as this repository's own benchmarks do) and then build
/// perfectly well under an abort strategy. Rejecting those would be a false
/// positive, and telling them apart needs per-target metadata this wrapper
/// does not read; the observed-strategy check after the build is what keeps
/// a mixed configuration from being *reported* either way.
pub(crate) const TEST_HARNESS_SELECTORS: &[&str] = &["--tests", "--all-targets"];

/// The Cargo subcommands the analysis can run under. `check` is deliberately
/// not one: it never reaches codegen, so the mono graph is not populated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CargoCommand {
    Build,
    Test,
}

impl CargoCommand {
    pub(crate) fn parse(name: &str) -> Result<Self> {
        ensure!(
            name != "check",
            "`cargo check` is unsupported because it does not produce a monomorphized instance graph"
        );
        match name {
            "build" => Ok(Self::Build),
            "test" => Ok(Self::Test),
            other => bail!("unsupported Cargo command `{other}`; expected `build` or `test`"),
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Test => "test",
        }
    }
}

/// Which standard library the checked build links against, and whether
/// panic paths are checked. `Strict` implies a rebuilt sysroot and checked
/// panic paths together, which is why this is one enum and not three flags:
/// "strict without build-std" and "panic paths checked against a sysroot
/// whose panics were not rebuilt to abort" are not representable invocations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StdMode {
    Precompiled,
    BuildStd,
    Strict,
}

impl StdMode {
    /// Whether the sysroot is rebuilt in this build. This, not the panic
    /// strategy, is what makes an observed `ImmediateAbort` coherent: a
    /// hand-set strategy plus plain `--build-std` rebuilds std under the same
    /// inherited flags, so both the pre-flight guard and the post-build mix
    /// check key off it.
    pub(crate) fn rebuilds_std(self) -> bool {
        self != Self::Precompiled
    }

    /// Whether this run injects `-Cpanic=immediate-abort` (and so cannot
    /// build a libtest harness).
    pub(crate) fn is_strict(self) -> bool {
        self == Self::Strict
    }

    /// The analysis mode the driver is told to run in (ADR 0008).
    pub(crate) fn panic_paths(self) -> PanicPaths {
        match self {
            Self::Precompiled | Self::BuildStd => PanicPaths::OutOfScope,
            Self::Strict => PanicPaths::Checked,
        }
    }
}

/// Which crates the driver instruments, and therefore which of Cargo's two
/// wrapper slots it occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    Workspace,
    AllCrates,
}

impl Scope {
    /// `(variable, value)` pairs for both wrapper slots. The slot the checker
    /// is not using is blanked rather than removed: an empty value overrides
    /// the matching config key (verified on the pinned Cargo), so a wrapper
    /// configured after the pre-flight check cannot slip in front of the
    /// driver.
    pub(crate) fn wrapper_env(self, driver: &Path) -> [(&'static str, OsString); 2] {
        let driver = driver.as_os_str().to_owned();
        match self {
            Self::AllCrates => [
                ("RUSTC_WRAPPER", driver),
                ("RUSTC_WORKSPACE_WRAPPER", OsString::new()),
            ],
            Self::Workspace => [
                ("RUSTC_WORKSPACE_WRAPPER", driver),
                ("RUSTC_WRAPPER", OsString::new()),
            ],
        }
    }

    pub(crate) fn driver_flag(self) -> &'static str {
        match self {
            Self::AllCrates => "1",
            Self::Workspace => "0",
        }
    }
}

/// What the command line asked for, before the environment is consulted.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Invocation {
    std_mode: StdMode,
    scope: Scope,
    warn_only: bool,
    // A set, so "sorted and deduplicated" holds by construction wherever
    // roots are added (flags, `NO_ALLOC_ROOTS`).
    roots: BTreeSet<String>,
    command: CargoCommand,
    cargo_args: Vec<String>,
}

/// An [`Invocation`] with the environment's say (`NO_ALLOC_WARN_ONLY`,
/// `NO_ALLOC_ROOTS`) folded in. A separate type so that nothing downstream can
/// be handed the half-resolved command line, and `run` never mutates what it
/// parsed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Config {
    pub(crate) std_mode: StdMode,
    pub(crate) scope: Scope,
    pub(crate) warn_only: bool,
    pub(crate) roots: BTreeSet<String>,
    pub(crate) command: CargoCommand,
    pub(crate) cargo_args: Vec<String>,
}

impl Config {
    /// The environment can only add: switch warn-only on (exactly `"1"`, like
    /// the driver reads it) and contribute more roots, never remove either.
    /// Values come in as arguments so this stays free of process state.
    pub(crate) fn resolve(
        invocation: Invocation,
        warn_only_env: Option<&str>,
        roots_env: Option<&str>,
    ) -> Self {
        let Invocation {
            std_mode,
            scope,
            warn_only,
            mut roots,
            command,
            cargo_args,
        } = invocation;
        if let Some(existing) = roots_env {
            roots.extend(parse_root_spec(existing));
        }
        Self {
            std_mode,
            scope,
            warn_only: warn_only || warn_only_env == Some("1"),
            roots,
            command,
            cargo_args,
        }
    }
}

// `--help`/`--version` short-circuit before an `Invocation` can even be
// formed (e.g. `--root` with no path would otherwise reject them), so they
// need their own variants rather than boolean fields on `Invocation`.
pub(crate) enum ParsedArgs {
    Help,
    Version,
    Run(Invocation),
}

/// One `--root` value: a comma-separated list, split by the same function
/// that reads `NO_ALLOC_ROOTS` so the two spellings cannot disagree. That
/// function silently drops empty entries, which is right for an environment
/// fallback but would hide a typo (`a,,b`) on the command line.
pub(crate) fn parse_root_value(value: &str) -> Result<Vec<String>> {
    ensure!(!value.trim().is_empty(), "--root requires a non-empty path");
    let roots = parse_root_spec(value);
    ensure!(
        roots.len() == value.split(',').count(),
        "--root `{value}` has an empty entry; separate roots with single commas"
    );
    Ok(roots)
}

pub(crate) fn parse_args(args: impl IntoIterator<Item = String>) -> Result<ParsedArgs> {
    let mut args: Vec<_> = args.into_iter().collect();
    if !args.is_empty() {
        args.remove(0);
    }
    // Cargo passes the external subcommand name as argv[1].
    if args.first().is_some_and(|arg| arg == "no-alloc") {
        args.remove(0);
    }

    let mut std_mode = StdMode::Precompiled;
    let mut scope = Scope::Workspace;
    let mut warn_only = false;
    let mut roots = BTreeSet::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--" => {
                index += 1;
                break;
            }
            // Never downgrades `--strict`, whichever order they come in.
            "--build-std" => {
                if std_mode == StdMode::Precompiled {
                    std_mode = StdMode::BuildStd;
                }
            }
            // Implies --build-std: the panic strategy has to match all the
            // way down, and the precompiled sysroot was not built with it.
            "--strict" => std_mode = StdMode::Strict,
            "--all-crates" => scope = Scope::AllCrates,
            "--warn-only" => warn_only = true,
            // A cargo subcommand is expected to answer these regardless of
            // what else is on the line, so they exit before anything past
            // this point (including "-- <bad cargo args>") is even parsed.
            "--help" | "-h" => return Ok(ParsedArgs::Help),
            "--version" | "-V" => return Ok(ParsedArgs::Version),
            "--root" => {
                index += 1;
                let spec = args
                    .get(index)
                    .context("--root requires a non-empty path")?;
                roots.extend(parse_root_value(spec)?);
            }
            value if value.starts_with("--root=") => {
                roots.extend(parse_root_value(value.trim_start_matches("--root="))?);
            }
            value if value.starts_with('-') => bail!("unknown checker option `{value}`"),
            _ => break,
        }
        index += 1;
    }

    let mut cargo = args[index..].to_vec();
    let command = if cargo
        .first()
        .is_none_or(|argument| argument.starts_with('-'))
    {
        CargoCommand::Build
    } else {
        CargoCommand::parse(&cargo.remove(0))?
    };
    // Caught here rather than left to rustc, which fails the build several
    // minutes into a sysroot rebuild with `building tests with panic=abort
    // is not supported without -Zpanic_abort_tests`. Cargo's own
    // `-Zpanic-abort-tests` does not help: it keys off the profile's panic
    // setting, and this strategy arrives through RUSTFLAGS instead.
    ensure!(
        !(std_mode.is_strict() && command == CargoCommand::Test),
        "`--strict` cannot be combined with `-- test`: rustc refuses to build a test harness \
         under the abort panic strategy `--strict` compiles with. Use `--strict -- build`, \
         or `-- test` without `--strict` (see docs/guarantees.md)"
    );
    for arg in split_at_separator(&cargo).0 {
        ensure!(
            arg != "--target"
                && !arg.starts_with("--target=")
                && arg != "--target-dir"
                && !arg.starts_with("--target-dir="),
            "Cargo target options conflict with no_alloc's dedicated checker target"
        );
        // The same wall `-- test` hits, reached through target selection
        // instead of the subcommand: these all compile libtest harnesses,
        // which rustc will not build under an abort panic strategy.
        let flag = arg.split('=').next().unwrap_or(arg);
        ensure!(
            !std_mode.is_strict() || !TEST_HARNESS_SELECTORS.contains(&flag),
            "`--strict` cannot build test-harness targets (`{flag}`): rustc refuses to build a \
             test harness under the abort panic strategy `--strict` compiles with. Drop the \
             target selection, or check those targets without `--strict`"
        );
    }
    Ok(ParsedArgs::Run(Invocation {
        std_mode,
        scope,
        warn_only,
        roots,
        command,
        cargo_args: cargo,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every pre-existing test wants an `Invocation`; only the new
    // help/version tests below care about the other `ParsedArgs` variants,
    // so they call `parse_args` directly instead of going through this.
    fn parse(args: &[&str]) -> Result<Invocation> {
        match parse_args(args.iter().map(|arg| (*arg).to_owned()))? {
            ParsedArgs::Run(inv) => Ok(inv),
            ParsedArgs::Help => bail!("expected an Invocation, got --help"),
            ParsedArgs::Version => bail!("expected an Invocation, got --version"),
        }
    }

    #[test]
    fn strips_actual_cargo_subcommand_argv() -> Result<()> {
        assert_eq!(
            parse(&["cargo-no-alloc", "no-alloc", "--", "build"])?.command,
            CargoCommand::Build
        );
        Ok(())
    }

    #[test]
    fn defaults_to_build() -> Result<()> {
        assert_eq!(parse(&["cargo-no-alloc"])?.command, CargoCommand::Build);
        let with_options = parse(&["cargo-no-alloc", "--", "--release"])?;
        assert_eq!(with_options.command, CargoCommand::Build);
        assert_eq!(with_options.cargo_args, ["--release"]);
        Ok(())
    }

    #[test]
    fn rejects_check_and_target_overrides() {
        assert!(parse(&["cargo-no-alloc", "--", "check"]).is_err());
        assert!(parse(&["cargo-no-alloc", "--", "build", "--target=x"]).is_err());
    }

    #[test]
    fn retains_test_runner_separator() -> Result<()> {
        let inv = parse(&["cargo-no-alloc", "--", "test", "case", "--", "--nocapture"])?;
        assert_eq!(inv.cargo_args, ["case", "--", "--nocapture"]);
        Ok(())
    }

    #[test]
    fn parses_checker_flags_and_roots() -> Result<()> {
        let inv = parse(&[
            "cargo-no-alloc",
            "no-alloc",
            "--warn-only",
            "--root",
            "z::f",
            "--root=a::f",
            "--",
            "test",
        ])?;
        assert!(inv.warn_only);
        assert_eq!(roots_of(&inv), ["a::f", "z::f"]);
        assert_eq!(inv.command, CargoCommand::Test);
        Ok(())
    }

    fn roots_of(inv: &Invocation) -> Vec<&str> {
        inv.roots.iter().map(String::as_str).collect()
    }

    #[test]
    fn the_environment_can_add_roots_and_warn_only_but_not_remove_them() -> Result<()> {
        let inv = || parse(&["cargo-no-alloc", "--root=b::f", "--", "build"]);
        let plain = Config::resolve(inv()?, None, None);
        assert!(!plain.warn_only);
        assert_eq!(plain.roots.len(), 1);

        let merged = Config::resolve(inv()?, Some("1"), Some("a::f, b::f ,c::f"));
        assert!(merged.warn_only);
        assert_eq!(
            merged.roots.iter().map(String::as_str).collect::<Vec<_>>(),
            ["a::f", "b::f", "c::f"]
        );

        // Only exactly "1" counts, as in the driver; anything else is no vote.
        for value in ["0", "true", "", "01"] {
            assert!(
                !Config::resolve(inv()?, Some(value), None).warn_only,
                "{value}"
            );
        }
        // A flag already given on the command line is never switched off.
        let flagged = parse(&["cargo-no-alloc", "--warn-only", "--", "build"])?;
        assert!(Config::resolve(flagged, Some("0"), None).warn_only);
        Ok(())
    }

    /// A `--root` value is a list, like `NO_ALLOC_ROOTS`: each entry is its
    /// own root, so each can be matched (and reported) individually.
    #[test]
    fn comma_separated_roots_are_split_in_both_spellings() -> Result<()> {
        let inv = parse(&[
            "cargo-no-alloc",
            "--root",
            " b::g , a::f",
            "--root=c::h,a::f",
            "--",
            "build",
        ])?;
        assert_eq!(roots_of(&inv), ["a::f", "b::g", "c::h"]);
        Ok(())
    }

    #[test]
    fn root_values_with_empty_entries_are_errors() {
        for value in ["a,,b", "a,", ",a", ",", " ", ""] {
            assert!(
                parse(&["cargo-no-alloc", "--root", value]).is_err(),
                "`--root {value}`"
            );
            let joined = format!("--root={value}");
            assert!(parse(&["cargo-no-alloc", &joined]).is_err(), "{joined}");
        }
    }

    #[test]
    fn scope_owns_which_wrapper_slot_is_blanked() {
        let driver = Path::new("/d/no-alloc-driver");
        let workspace = Scope::Workspace.wrapper_env(driver);
        assert_eq!(
            workspace,
            [
                (
                    "RUSTC_WORKSPACE_WRAPPER",
                    OsString::from("/d/no-alloc-driver")
                ),
                ("RUSTC_WRAPPER", OsString::new()),
            ]
        );
        let all = Scope::AllCrates.wrapper_env(driver);
        assert_eq!(
            all,
            [
                ("RUSTC_WRAPPER", OsString::from("/d/no-alloc-driver")),
                ("RUSTC_WORKSPACE_WRAPPER", OsString::new()),
            ]
        );
    }

    #[test]
    fn strict_implies_build_std_and_checked_panic_paths() -> Result<()> {
        let inv = parse(&["cargo-no-alloc", "--strict", "--", "build"])?;
        assert!(inv.std_mode.is_strict());
        assert_eq!(inv.std_mode.panic_paths(), PanicPaths::Checked);
        // Rebuilding the sysroot is not optional here: the precompiled one
        // was built with a different panic strategy, and mixing them is
        // exactly what this flag exists to avoid.
        assert!(inv.std_mode.rebuilds_std());
        // --build-std on its own changes neither the strategy nor the scope.
        let inv = parse(&["cargo-no-alloc", "--build-std", "--", "build"])?;
        assert!(inv.std_mode.rebuilds_std());
        assert!(!inv.std_mode.is_strict());
        assert_eq!(inv.std_mode.panic_paths(), PanicPaths::OutOfScope);
        // The default leaves panic paths out of scope (ADR 0008).
        let inv = parse(&["cargo-no-alloc", "--", "build"])?;
        assert_eq!(inv.std_mode.panic_paths(), PanicPaths::OutOfScope);
        // Flag order must not matter: `--build-std` never downgrades.
        for order in [["--strict", "--build-std"], ["--build-std", "--strict"]] {
            let inv = parse(&["cargo-no-alloc", order[0], order[1], "--", "build"])?;
            assert_eq!(inv.std_mode, StdMode::Strict, "{order:?}");
        }
        Ok(())
    }

    /// `--immediate-abort` was removed, not aliased, so the old spelling
    /// must fail loudly rather than run the weaker default.
    #[test]
    fn the_removed_immediate_abort_flag_is_an_unknown_option() {
        let error = parse(&["cargo-no-alloc", "--immediate-abort", "--", "build"])
            .expect_err("--immediate-abort no longer exists");
        assert!(error.to_string().contains("--immediate-abort"), "{error}");
    }

    #[test]
    fn strict_rejects_test_mode() {
        let error = parse(&["cargo-no-alloc", "--strict", "--", "test"])
            .expect_err("--strict -- test cannot build");
        // The point of rejecting at parse time is the message: rustc's own
        // failure arrives after a full sysroot rebuild and names an
        // unrelated-looking flag.
        assert!(error.to_string().contains("-- build"), "{error}");
        // Neither half is a problem on its own.
        assert!(parse(&["cargo-no-alloc", "--strict", "--", "build"]).is_ok());
        assert!(parse(&["cargo-no-alloc", "--", "test"]).is_ok());
    }

    #[test]
    fn strict_rejects_test_harness_target_selection() -> Result<()> {
        // `build --tests` reaches the same rustc failure as `-- test`, just
        // through target selection, and just as far into a sysroot rebuild.
        for selection in [vec!["--tests"], vec!["--all-targets"]] {
            let mut args = vec!["cargo-no-alloc", "--strict", "--", "build"];
            args.extend(selection.iter().copied());
            let error = parse(&args).expect_err(&format!("{selection:?} must be rejected"));
            assert!(error.to_string().contains("test-harness"), "{error}");
            // Only `--strict` has the problem; plain runs still build them.
            let mut plain = vec!["cargo-no-alloc", "--", "build"];
            plain.extend(selection.iter().copied());
            assert!(parse(&plain).is_ok(), "{selection:?} without the flag");
        }
        // Selections that name one target are *not* rejected: it may set
        // `harness = false` and build fine, and this wrapper does not read
        // per-target metadata to tell. The observed-strategy check after the
        // build is what protects the report either way.
        for selection in [
            vec!["--bench", "throughput"],
            vec!["--benches"],
            vec!["--test", "ui"],
        ] {
            let mut args = vec!["cargo-no-alloc", "--strict", "--", "build"];
            args.extend(selection.iter().copied());
            assert!(parse(&args).is_ok(), "{selection:?} must be allowed");
        }
        // Target selections that build no harness stay accepted.
        assert!(parse(&["cargo-no-alloc", "--strict", "--", "build", "--lib"]).is_ok());
        assert!(parse(&["cargo-no-alloc", "--strict", "--", "build", "--examples"]).is_ok());
        // Arguments after the test-runner separator are not target selection.
        parse(&["cargo-no-alloc", "--strict", "--", "build", "--", "--tests"])?;
        Ok(())
    }

    #[test]
    fn rejects_malformed_checker_arguments() {
        assert!(parse(&["cargo-no-alloc", "--root"]).is_err());
        assert!(parse(&["cargo-no-alloc", "--root="]).is_err());
        assert!(parse(&["cargo-no-alloc", "--unknown"]).is_err());
        assert!(parse(&["cargo-no-alloc", "--", "run"]).is_err());
        assert!(parse(&["cargo-no-alloc", "--", "build", "--target", "x"]).is_err());
        assert!(parse(&["cargo-no-alloc", "--", "build", "--target-dir=x"]).is_err());
    }

    #[test]
    fn root_equals_and_direct_command_are_supported() -> Result<()> {
        let invocation = parse(&[
            "cargo-no-alloc",
            "--build-std",
            "--all-crates",
            "--root=z::f",
            "build",
        ])?;
        assert_eq!(invocation.std_mode, StdMode::BuildStd);
        assert_eq!(invocation.scope, Scope::AllCrates);
        assert_eq!(roots_of(&invocation), ["z::f"]);
        Ok(())
    }

    #[test]
    fn help_flag_short_and_long_are_recognized() -> Result<()> {
        for args in [["cargo-no-alloc", "--help"], ["cargo-no-alloc", "-h"]] {
            assert!(matches!(
                parse_args(args.iter().map(|arg| (*arg).to_owned()))?,
                ParsedArgs::Help
            ));
        }
        Ok(())
    }

    #[test]
    fn version_flag_short_and_long_are_recognized() -> Result<()> {
        for args in [["cargo-no-alloc", "--version"], ["cargo-no-alloc", "-V"]] {
            assert!(matches!(
                parse_args(args.iter().map(|arg| (*arg).to_owned()))?,
                ParsedArgs::Version
            ));
        }
        Ok(())
    }

    #[test]
    fn help_flag_short_circuits_before_other_argument_errors() -> Result<()> {
        // A trailing bare `--root` (no path) would reject on its own if
        // parsing reached it — --help must return before that happens.
        assert!(matches!(
            parse_args(
                ["cargo-no-alloc", "--warn-only", "--help", "--root"]
                    .iter()
                    .map(|arg| (*arg).to_owned())
            )?,
            ParsedArgs::Help
        ));
        Ok(())
    }

    #[test]
    fn help_text_documents_every_flag_parse_args_accepts() {
        // Kept as a content check (rather than duplicating the text) so the
        // help output can't silently drift from what `parse_args` parses.
        for flag in [
            "--all-crates",
            "--build-std",
            "--strict",
            "--warn-only",
            "--root",
            "--help",
            "-h",
            "--version",
            "-V",
            "[build|test]",
        ] {
            assert!(HELP_TEXT.contains(flag), "help text missing `{flag}`");
        }
    }
}
