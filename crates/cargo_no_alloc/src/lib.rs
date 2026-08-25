use anyhow::{bail, ensure, Context, Result};
use fs2::FileExt;
use no_alloc_report::{parse_root_spec, PanicStrategy, Report, ReportFragment, Verdict};
use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

// Shared with `build.rs` so the pin can't drift between the build-time
// check (the toolchain compiling this binary) and the run-time check below
// (the toolchain of the project being analyzed).
include!("toolchain_spec.rs");

// A cargo subcommand is expected to answer `--help`/`-h`; kept as a plain
// string (rather than built from `Invocation`'s fields) so it reads as the
// user-facing contract, not an implementation detail that happens to leak.
const HELP_TEXT: &str = "\
cargo no-alloc [OPTIONS] -- [build|test] [CARGO_ARGS...]

Statically checks that #[no_alloc]-marked function instances cannot reach
the global allocator, then runs the underlying Cargo command (`build` is
used when [build|test] is omitted; `check` is rejected because it never
produces the monomorphized instance graph the analysis runs on).

Options:
      --all-crates   Instrument every crate in the build (RUSTC_WRAPPER),
                     not just workspace members (RUSTC_WORKSPACE_WRAPPER)
      --build-std    Pass -Zbuild-std to Cargo
      --immediate-abort
                     Compile everything, std included, with
                     -Cpanic=immediate-abort so panic paths lower to a bare
                     abort() and can be checked instead of rejected.
                     Implies --build-std; cannot be combined with `test`.
                     See docs/iterators.md
      --warn-only    Report findings on stderr without failing the build
      --root PATH    Additionally check an unannotated function by its
                     canonical path (repeatable)
  -h, --help         Print this help and exit
  -V, --version      Print version information and exit";

// Printed unconditionally (not just under --warn-only) because a clean exit
// with zero roots checked is not a passing result, just an unverified one —
// see the README's "By default only workspace members are instrumented".
const ZERO_ROOTS_WARNING: &str = "warning: no_alloc checked 0 root instances \
— nothing was analyzed. This usually means either no `#[no_alloc]` marker \
was reached while building, or the marker lives in a crate outside the \
workspace (pass --all-crates to instrument it).";

/// Added by `--immediate-abort`. `-Cpanic=immediate-abort` is still
/// unstable, hence the accompanying `-Zunstable-options`. Passing it here
/// rather than asking users to set `[profile.dev] panic = "immediate-abort"`
/// keeps the checked crate's own manifest stable-buildable: the manifest key
/// requires `cargo-features = ["panic-immediate-abort"]`, which makes the
/// whole manifest nightly-only (ADR 0002's zero-footprint rule).
const IMMEDIATE_ABORT_RUSTFLAGS: &[&str] = &["-Zunstable-options", "-Cpanic=immediate-abort"];

/// Cargo target selections that always pull in a libtest harness, which
/// cannot be built under `--immediate-abort` (see the `-- test` rejection).
///
/// Only these two: both include the unit-test targets of the lib and bins,
/// which use libtest whatever the manifest says. `--test`/`--bench` name one
/// target and `--benches` selects only bench targets — any of which may set
/// `harness = false` (as this repository's own benchmarks do) and then build
/// perfectly well under an abort strategy. Rejecting those would be a false
/// positive, and telling them apart needs per-target metadata this wrapper
/// does not read; the observed-strategy check after the build is what keeps
/// a mixed configuration from being *reported* either way.
const TEST_HARNESS_SELECTORS: &[&str] = &["--tests", "--all-targets"];

const REQUIRED_RUSTFLAGS: &[&str] = &[
    "--cfg=no_alloc_check",
    "--check-cfg=cfg(no_alloc_check)",
    "-Zcrate-attr=feature(register_tool)",
    "-Zcrate-attr=register_tool(no_alloc_tool)",
    "-Zalways-encode-mir",
];
const CACHE_DIR_TAG: &str = "Signature: 8a477f597d28d172789f06886806bc55\n\
# This file is a cache directory tag created by cargo.\n\
# For information about cache directory tags see https://bford.info/cachedir/\n";

#[derive(Debug, PartialEq, Eq)]
struct Invocation {
    build_std: bool,
    immediate_abort: bool,
    all_crates: bool,
    warn_only: bool,
    roots: Vec<String>,
    command: String,
    cargo_args: Vec<String>,
}

// `--help`/`--version` short-circuit before an `Invocation` can even be
// formed (e.g. `--root` with no path would otherwise reject them), so they
// need their own variants rather than boolean fields on `Invocation`.
enum ParsedArgs {
    Help,
    Version,
    Run(Invocation),
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<ParsedArgs> {
    let mut args: Vec<_> = args.into_iter().collect();
    if !args.is_empty() {
        args.remove(0);
    }
    // Cargo passes the external subcommand name as argv[1].
    if args.first().is_some_and(|arg| arg == "no-alloc") {
        args.remove(0);
    }

    let mut build_std = false;
    let mut immediate_abort = false;
    let mut all_crates = false;
    let mut warn_only = false;
    let mut roots = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--" => {
                index += 1;
                break;
            }
            "--build-std" => build_std = true,
            // Implies --build-std: the panic strategy has to match all the
            // way down, and the precompiled sysroot was not built with it.
            "--immediate-abort" => {
                immediate_abort = true;
                build_std = true;
            }
            "--all-crates" => all_crates = true,
            "--warn-only" => warn_only = true,
            // A cargo subcommand is expected to answer these regardless of
            // what else is on the line, so they exit before anything past
            // this point (including "-- <bad cargo args>") is even parsed.
            "--help" | "-h" => return Ok(ParsedArgs::Help),
            "--version" | "-V" => return Ok(ParsedArgs::Version),
            "--root" => {
                index += 1;
                let root = args
                    .get(index)
                    .context("--root requires a non-empty path")?;
                ensure!(!root.trim().is_empty(), "--root requires a non-empty path");
                roots.push(root.trim().to_owned());
            }
            value if value.starts_with("--root=") => {
                let root = value.trim_start_matches("--root=").trim();
                ensure!(!root.is_empty(), "--root requires a non-empty path");
                roots.push(root.to_owned());
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
        "build".to_owned()
    } else {
        cargo.remove(0)
    };
    ensure!(
        command != "check",
        "`cargo check` is unsupported because it does not produce a monomorphized instance graph"
    );
    ensure!(
        matches!(command.as_str(), "build" | "test"),
        "unsupported Cargo command `{command}`; expected `build` or `test`"
    );
    // Caught here rather than left to rustc, which fails the build several
    // minutes into a sysroot rebuild with `building tests with panic=abort
    // is not supported without -Zpanic_abort_tests`. Cargo's own
    // `-Zpanic-abort-tests` does not help: it keys off the profile's panic
    // setting, and this strategy arrives through RUSTFLAGS instead.
    ensure!(
        !(immediate_abort && command == "test"),
        "`--immediate-abort` cannot be combined with `-- test`: rustc refuses to build a \
         test harness under an abort panic strategy. Use `-- build`, which is the supported \
         mode for meaningful results anyway (see README, \"Guarantee and limitations\")"
    );
    let cargo_option_end = cargo
        .iter()
        .position(|arg| arg == "--")
        .unwrap_or(cargo.len());
    for arg in &cargo[..cargo_option_end] {
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
            !immediate_abort || !TEST_HARNESS_SELECTORS.contains(&flag),
            "`--immediate-abort` cannot build test-harness targets (`{flag}`): rustc refuses \
             to build a test harness under an abort panic strategy. Drop the target selection, \
             or check those targets without `--immediate-abort`"
        );
    }
    roots.sort();
    roots.dedup();
    Ok(ParsedArgs::Run(Invocation {
        build_std,
        immediate_abort,
        all_crates,
        warn_only,
        roots,
        command,
        cargo_args: cargo,
    }))
}

fn host_triple() -> Result<String> {
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let output = Command::new(&rustc)
        .args(["--print", "host-tuple"])
        .output()
        .with_context(|| format!("failed to run `{rustc} --print host-tuple`"))?;
    ensure!(
        output.status.success(),
        "`{rustc} --print host-tuple` exited with {}",
        output.status
    );
    Ok(String::from_utf8(output.stdout)
        .context("host-tuple output is not UTF-8")?
        .trim()
        .to_owned())
}

// Checks the toolchain of the project being analyzed, mirroring build.rs's
// compile-time check on the toolchain that built this binary. Without this,
// a project pinned to a different toolchain (e.g. via rust-toolchain.toml)
// resolves the wrong sysroot in `sysroot_lib_dir` below, which otherwise
// only ever surfaces many steps later as a raw dynamic-linker error.
// `rustc` is threaded in explicitly (rather than read from the environment
// here) so this predicate can be unit tested against a fake rustc without
// touching process-wide env state.
fn verify_pinned_toolchain(rustc: &str) -> Result<()> {
    let output = Command::new(rustc)
        .arg("-vV")
        .output()
        .with_context(|| format!("failed to run `{rustc} -vV`"))?;
    ensure!(
        output.status.success(),
        "`{rustc} -vV` exited with {}",
        output.status
    );
    let version = String::from_utf8(output.stdout).context("rustc version is not UTF-8")?;
    let host = version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .context("rustc did not report a host triple")?;
    ensure!(
        is_pinned_toolchain(&version, host),
        toolchain_mismatch_message(&version)
    );
    Ok(())
}

fn sysroot_lib_dir() -> Result<PathBuf> {
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let output = Command::new(&rustc)
        .args(["--print", "sysroot"])
        .output()
        .with_context(|| format!("failed to run `{rustc} --print sysroot`"))?;
    ensure!(
        output.status.success(),
        "`{rustc} --print sysroot` exited with {}",
        output.status
    );
    let sysroot = String::from_utf8(output.stdout)
        .context("sysroot path is not valid UTF-8")?
        .trim()
        .to_owned();
    Ok(PathBuf::from(sysroot).join("lib"))
}

// `no-alloc-driver` dynamically links `librustc_driver-*.so`/`libLLVM.so`,
// which live in the sysroot rather than next to the binary. A binary built
// on one machine (e.g. CI) and run on another can't rely on a path baked in
// at build time, so this is resolved fresh from whatever `rustc` is active
// for the invoking project and threaded through as `LD_LIBRARY_PATH`.
fn combined_ld_library_path(sysroot_lib: &Path, existing: Option<&std::ffi::OsStr>) -> OsString {
    match existing {
        Some(existing) if !existing.is_empty() => {
            let mut combined = sysroot_lib.as_os_str().to_owned();
            combined.push(":");
            combined.push(existing);
            combined
        }
        _ => sysroot_lib.as_os_str().to_owned(),
    }
}

fn find_driver() -> Result<PathBuf> {
    let exe = env::current_exe().context("failed to resolve current executable")?;
    let sibling = exe
        .parent()
        .context("current executable has no parent")?
        .join("no-alloc-driver");
    Ok(if sibling.is_file() {
        sibling
    } else {
        PathBuf::from("no-alloc-driver")
    })
}

/// The value of the last `-C panic=...` setting in a token stream, in the
/// order rustc would see them. `-Cpanic` is last-wins, so a stream ending in
/// `-Cpanic=unwind` compiles under `unwind` even if `immediate-abort`
/// appeared earlier — checking "does this value appear anywhere" instead of
/// "which one wins" flagged an effectively-`unwind` environment as if it
/// selected immediate-abort, which is how this was found.
///
/// Handles both the joined spelling (`-Cpanic=val`, `--codegen=panic=val`)
/// and the two-token form `rustc -C help` also documents (`-C panic=val`,
/// `--codegen panic=val`) — both reach rustc identically. A bare `-C`/
/// `--codegen` matches only the two-token form's first slot, checked before
/// the joined-prefix strip so it can't be mistaken for a (nonsensical) empty
/// joined value.
fn last_panic_setting<'a>(tokens: &[&'a str]) -> Option<&'a str> {
    let mut last = None;
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        if matches!(token, "-C" | "--codegen") {
            if let Some(value) = tokens
                .get(index + 1)
                .and_then(|next| next.strip_prefix("panic="))
            {
                last = Some(value);
                index += 1;
            }
        } else if let Some(rest) = token
            .strip_prefix("-C")
            .or_else(|| token.strip_prefix("--codegen"))
        {
            if let Some(value) = rest.trim_start_matches('=').strip_prefix("panic=") {
                last = Some(value);
            }
        }
        index += 1;
    }
    last
}

/// Whether the caller's own flags currently select the immediate-abort
/// panic strategy — the *last* one, since that is the one rustc uses. Split
/// out from the environment so it can be tested directly:
/// `CARGO_ENCODED_RUSTFLAGS` is `\x1f`-separated and takes precedence over
/// `RUSTFLAGS`, exactly as Cargo reads them.
fn declares_immediate_abort(encoded: Option<&str>, plain: Option<&str>) -> bool {
    let raw: Vec<&str> = match encoded {
        Some(encoded) => encoded.split('\x1f').collect(),
        None => plain.unwrap_or_default().split_whitespace().collect(),
    };
    // An encoded entry can itself carry the space, so flatten before pairing
    // rather than trusting the separator to have split every token.
    let tokens: Vec<&str> = raw
        .iter()
        .flat_map(|token| token.split_whitespace())
        .collect();
    last_panic_setting(&tokens) == Some("immediate-abort")
}

/// Whether the build the driver just ran mixes an immediate-abort crate
/// with a standard library that was not rebuilt to match.
///
/// What makes an observed `ImmediateAbort` strategy safe is that the
/// sysroot was rebuilt *in this build* — which is what `--build-std` does:
/// Cargo applies the same profile and the same `RUSTFLAGS` to those units
/// too. So `--build-std` with the strategy set by hand (the manifest, a
/// `config.toml`, `--profile`) is coherent and not flagged; only an
/// immediate-abort crate built against the untouched precompiled sysroot
/// is a mix worth rejecting.
///
/// `any_immediate_abort` comes from scanning the raw fragments
/// (`aggregate`), not from `report.panic_strategy`: the merged field goes to
/// `None` on any disagreement between fragments, which would silently hide
/// exactly the mix this check exists to catch (a target-specific config
/// selecting the strategy for the checked crate while a wrapped host unit
/// compiles under something else).
fn mixed_sysroot_panic_strategy(build_std: bool, any_immediate_abort: bool) -> bool {
    !build_std && any_immediate_abort
}

fn add_required_rustflags(command: &mut Command, immediate_abort: bool) {
    // Two passes, and the order matters. The always-required flags are added
    // only when absent, so an inherited copy is left alone. The panic
    // strategy is then appended unconditionally, because `-Cpanic` is
    // last-wins: "already present somewhere" is not the same as "in effect",
    // and an inherited `-Cpanic=immediate-abort ... -Cpanic=unwind` would
    // otherwise satisfy a presence test while the build ran under unwind,
    // with `--immediate-abort` quietly not happening. Repeating the flag is
    // harmless; being outvoted by it is not.
    let panic_strategy: &[&str] = if immediate_abort {
        IMMEDIATE_ABORT_RUSTFLAGS
    } else {
        &[]
    };
    if let Some(encoded) = env::var_os("CARGO_ENCODED_RUSTFLAGS") {
        let mut value = encoded.to_string_lossy().into_owned();
        let existing: HashSet<String> = value.split('\x1f').map(str::to_owned).collect();
        let push = |flag: &str, value: &mut String| {
            if !value.is_empty() {
                value.push('\x1f');
            }
            value.push_str(flag);
        };
        for flag in REQUIRED_RUSTFLAGS {
            if !existing.contains(*flag) {
                push(flag, &mut value);
            }
        }
        for flag in panic_strategy {
            push(flag, &mut value);
        }
        command.env("CARGO_ENCODED_RUSTFLAGS", value);
    } else {
        let mut value = env::var("RUSTFLAGS").unwrap_or_default();
        let push = |flag: &str, value: &mut String| {
            if !value.is_empty() {
                value.push(' ');
            }
            value.push_str(flag);
        };
        for flag in REQUIRED_RUSTFLAGS {
            if !value.split_whitespace().any(|existing| existing == *flag) {
                push(flag, &mut value);
            }
        }
        for flag in panic_strategy {
            push(flag, &mut value);
        }
        command.env("RUSTFLAGS", value);
    }
}

fn cargo_status(mut command: Command) -> Result<ExitStatus> {
    command.status().context("failed to spawn Cargo")
}

fn clear_checker_artifacts(
    cargo: &OsString,
    inv: &Invocation,
    target_dir: &Path,
    host: &str,
) -> Result<()> {
    let mut clean = Command::new(cargo);
    clean.arg("clean");
    if !inv.all_crates {
        clean.arg("--workspace");
    }
    clean
        .arg("--target")
        .arg(host)
        .arg("--target-dir")
        .arg(target_dir);
    let status = cargo_status(clean)?;
    ensure!(status.success(), "Cargo clean exited with {status}");
    Ok(())
}

/// `Report::merge`'s agreed-strategy answer is the right thing for
/// `report.json` to state (ADR 0006) — it is silent rather than guessing
/// when fragments disagree. It is the wrong question for this wrapper's own
/// safety net, though: a target-specific `.cargo/config.toml` can compile
/// the checked crate under `ImmediateAbort` while a wrapped host unit (a
/// build script with a checked root of its own) compiles under `Unwind`,
/// and the disagreement collapses the merged strategy to `None` — silently
/// clearing the one signal `mixed_sysroot_panic_strategy` needs to catch
/// exactly that case. So this is answered directly from the fragments,
/// before merging can lose it: "did *any* verdict-bearing fragment observe
/// `ImmediateAbort`", regardless of what the others said.
fn aggregate(fragment_dir: &Path, requested: &[String]) -> Result<(Report, bool)> {
    let mut reports = Vec::new();
    let mut matched = HashSet::new();
    if fragment_dir.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(fragment_dir)?.collect::<std::io::Result<_>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let fragment: ReportFragment = serde_json::from_reader(File::open(entry.path())?)
                .with_context(|| format!("failed to parse {}", entry.path().display()))?;
            matched.extend(fragment.matched_root_specs);
            reports.push(fragment.report);
        }
    }
    let any_immediate_abort = reports.iter().any(|report| {
        report.checked_an_instance() && report.panic_strategy == Some(PanicStrategy::ImmediateAbort)
    });
    let mut report = Report::merge(reports);
    for root in requested {
        if !matched.contains(root) {
            report.selection_errors.push(format!(
                "requested root `{root}` did not match any item in the build"
            ));
        }
    }
    report.selection_errors.sort();
    report.selection_errors.dedup();
    Ok((report, any_immediate_abort))
}

pub fn run(args: impl IntoIterator<Item = String>) -> Result<()> {
    let mut inv = match parse_args(args)? {
        // Neither prints anything about the pinned toolchain or spawns a
        // build, so both work even when the active toolchain is wrong.
        ParsedArgs::Help => {
            println!("{HELP_TEXT}");
            return Ok(());
        }
        ParsedArgs::Version => {
            println!("cargo-no-alloc {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        ParsedArgs::Run(inv) => inv,
    };
    // Checked before anything else touches Cargo: a project pinned to the
    // wrong toolchain otherwise fails much further down, in
    // `sysroot_lib_dir`, as a raw dynamic-linker error instead of this
    // actionable message.
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    verify_pinned_toolchain(&rustc)?;
    if env::var("NO_ALLOC_WARN_ONLY").as_deref() == Ok("1") {
        inv.warn_only = true;
    }
    if env::var_os("RUSTC_WRAPPER").is_some() || env::var_os("RUSTC_WORKSPACE_WRAPPER").is_some() {
        bail!(
            "existing RUSTC_WRAPPER/RUSTC_WORKSPACE_WRAPPER configuration is incompatible with cargo-no-alloc"
        );
    }
    if let Ok(existing) = env::var("NO_ALLOC_ROOTS") {
        inv.roots.extend(parse_root_spec(&existing));
        inv.roots.sort();
        inv.roots.dedup();
    }

    let cwd = env::current_dir().context("failed to determine current directory")?;
    let target_dir = cwd.join("target/no-alloc");
    let fragment_dir = target_dir.join("fragments");
    fs::create_dir_all(cwd.join("target"))?;
    let lock = File::create(cwd.join("target/no-alloc.lock"))?;
    lock.lock_exclusive()
        .context("failed to lock target/no-alloc.lock")?;

    // The strategy can arrive through the caller's own flags instead of the
    // checker option, and then nothing rebuilds the sysroot: the crate
    // compiles under immediate-abort while std keeps the panic runtime it
    // was precompiled with, and the report would claim `immediate_abort` for
    // a build where std's panic paths are not compiled that way at all.
    // `--immediate-abort` exists precisely because that combination has to
    // be applied to both halves (ADR 0006).
    ensure!(
        inv.immediate_abort
            || !declares_immediate_abort(
                env::var("CARGO_ENCODED_RUSTFLAGS").ok().as_deref(),
                env::var("RUSTFLAGS").ok().as_deref(),
            ),
        "the environment's Rust flags already select `-Cpanic=immediate-abort`, but \
         `--immediate-abort` was not passed. That strategy only means what the report says \
         it means if the standard library is rebuilt with it too; pass `--immediate-abort`, \
         which does that, instead of setting the flag by hand"
    );

    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let host = host_triple()?;
    // The fragment directory exists before Cargo first sees this custom
    // target, so create Cargo's safety tag before asking `cargo clean` to
    // manage it on this and later runs.
    fs::create_dir_all(&target_dir)?;
    fs::write(target_dir.join("CACHEDIR.TAG"), CACHE_DIR_TAG)?;
    clear_checker_artifacts(&cargo, &inv, &target_dir, &host)?;
    if fragment_dir.exists() {
        fs::remove_dir_all(&fragment_dir)?;
    }
    fs::create_dir_all(&fragment_dir)?;

    let mut command = Command::new(&cargo);
    command.arg(&inv.command);
    let split = inv
        .cargo_args
        .iter()
        .position(|arg| arg == "--")
        .unwrap_or(inv.cargo_args.len());
    command.args(&inv.cargo_args[..split]);
    command
        .arg("--target")
        .arg(&host)
        .arg("--target-dir")
        .arg(&target_dir);
    if inv.build_std {
        command.arg("-Zbuild-std");
    }
    command.args(&inv.cargo_args[split..]);
    add_required_rustflags(&mut command, inv.immediate_abort);
    command.env("NO_ALLOC_FRAGMENT_DIR", &fragment_dir);
    command.env("NO_ALLOC_ROOTS", inv.roots.join(","));
    if inv.warn_only {
        command.env("NO_ALLOC_WARN_ONLY", "1");
    } else {
        command.env_remove("NO_ALLOC_WARN_ONLY");
    }
    let driver = find_driver()?;
    let sysroot_lib = sysroot_lib_dir()?;
    command.env(
        "LD_LIBRARY_PATH",
        combined_ld_library_path(&sysroot_lib, env::var_os("LD_LIBRARY_PATH").as_deref()),
    );
    if inv.all_crates {
        command.env("RUSTC_WRAPPER", driver);
    } else {
        command.env("RUSTC_WORKSPACE_WRAPPER", driver);
    }

    let cargo_status = cargo_status(command)?;
    let (report, any_immediate_abort) = aggregate(&fragment_dir, &inv.roots)?;
    report
        .write_to_file(&target_dir.join("report.json"))
        .context("failed to write final report.json")?;
    for error in &report.selection_errors {
        eprintln!(
            "{}: {error}",
            if inv.warn_only { "warning" } else { "error" }
        );
    }
    let failures = report
        .roots
        .iter()
        .filter(|root| !matches!(root.verdict, Verdict::Pass | Verdict::NotInstantiated))
        .count();
    eprintln!(
        "no_alloc: checked {} root instance(s), {failures} finding(s)",
        report.roots.len()
    );
    // 0 roots is a silent-pass hazard, not a clean pass: it exits 0 and
    // looks identical to "everything checked out" unless called out here.
    if report.roots.is_empty() {
        eprintln!("{ZERO_ROOTS_WARNING}");
    }
    // The pre-flight guard reads the environment, and the strategy can also
    // arrive from the manifest (`cargo-features = ["panic-immediate-abort"]`
    // plus a profile), `config.toml`, or `--profile`. This one asks the
    // build itself: the driver recorded the strategy rustc actually used, so
    // a mixed configuration is caught however it was selected. The report is
    // written first, so the evidence survives the failure.
    ensure!(
        !mixed_sysroot_panic_strategy(inv.build_std, any_immediate_abort),
        "this build compiled with `-Cpanic=immediate-abort` against the precompiled standard \
         library, which carries its own panic runtime — so the report's verdicts would claim \
         panic paths that lower to `abort` when std's do not. Pass `--immediate-abort`, which \
         applies the strategy to both halves, or `--build-std` if you are setting the \
         strategy yourself (see docs/iterators.md)"
    );
    if !cargo_status.success() {
        bail!("Cargo exited with {cargo_status}");
    }
    if !inv.warn_only && !report.is_success() {
        bail!("no_alloc check failed");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    /// Serializes the tests that write a fake `rustc` and then execute it.
    ///
    /// Writing an executable and running it is not thread-safe against a
    /// *sibling* doing the same: the other test's `Command::spawn` inherits
    /// the still-open write fd, and the exec then fails with `ETXTBSY`
    /// ("Text file busy"). It reproduced about half the time here, as
    /// whichever test lost the race failing with a spawn error instead of
    /// the verdict it asserts on.
    static FAKE_RUSTC: std::sync::Mutex<()> = std::sync::Mutex::new(());

    use super::*;
    use no_alloc_report::RootVerdict;
    use std::os::unix::fs::PermissionsExt;

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
    fn ld_library_path_without_existing_value() {
        assert_eq!(
            combined_ld_library_path(Path::new("/fake/sysroot/lib"), None),
            OsString::from("/fake/sysroot/lib")
        );
    }

    #[test]
    fn ld_library_path_prepended_to_existing_value() {
        assert_eq!(
            combined_ld_library_path(
                Path::new("/fake/sysroot/lib"),
                Some(std::ffi::OsStr::new("/other/lib"))
            ),
            OsString::from("/fake/sysroot/lib:/other/lib")
        );
    }

    #[test]
    fn ld_library_path_treats_empty_existing_value_as_absent() {
        assert_eq!(
            combined_ld_library_path(
                Path::new("/fake/sysroot/lib"),
                Some(std::ffi::OsStr::new(""))
            ),
            OsString::from("/fake/sysroot/lib")
        );
    }

    #[test]
    fn strips_actual_cargo_subcommand_argv() -> Result<()> {
        assert_eq!(
            parse(&["cargo-no-alloc", "no-alloc", "--", "build"])?.command,
            "build"
        );
        Ok(())
    }

    #[test]
    fn defaults_to_build() -> Result<()> {
        assert_eq!(parse(&["cargo-no-alloc"])?.command, "build");
        let with_options = parse(&["cargo-no-alloc", "--", "--release"])?;
        assert_eq!(with_options.command, "build");
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
        assert_eq!(inv.roots, ["a::f", "z::f"]);
        assert_eq!(inv.command, "test");
        Ok(())
    }

    #[test]
    fn ambient_immediate_abort_is_recognized_in_either_flag_variable() {
        // Encoded takes precedence over plain, as in Cargo.
        assert!(declares_immediate_abort(
            Some("-Cpanic=immediate-abort"),
            None
        ));
        assert!(declares_immediate_abort(
            Some("--cfg=x\u{1f}-Cpanic=immediate-abort"),
            None
        ));
        assert!(declares_immediate_abort(
            None,
            Some("-Zunstable-options -Cpanic=immediate-abort")
        ));
        assert!(!declares_immediate_abort(
            Some("--cfg=x"),
            Some("-Cpanic=immediate-abort")
        ));
        // Neighbouring strategies are not this one.
        assert!(!declares_immediate_abort(None, Some("-Cpanic=abort")));
        assert!(!declares_immediate_abort(None, Some("-Cpanic=unwind")));
        assert!(!declares_immediate_abort(None, None));
    }

    /// `-C panic=val` is a documented spelling, and it reaches rustc the
    /// same way the joined one does — as two adjacent arguments in either
    /// variable, or as one encoded entry containing the space. Matching only
    /// `-Cpanic=…` let every split form through.
    #[test]
    fn ambient_immediate_abort_is_recognized_when_the_flag_is_split() {
        for plain in [
            "-Zunstable-options -C panic=immediate-abort",
            "-C panic=immediate-abort",
            "--codegen panic=immediate-abort",
            "--codegen=panic=immediate-abort",
        ] {
            assert!(declares_immediate_abort(None, Some(plain)), "{plain}");
        }
        for encoded in [
            "-Zunstable-options\u{1f}-C\u{1f}panic=immediate-abort",
            "-C panic=immediate-abort",
            "--codegen\u{1f}panic=immediate-abort",
        ] {
            assert!(declares_immediate_abort(Some(encoded), None), "{encoded}");
        }
        // A dangling `-C`, or one whose value is a different strategy, is not
        // a match — including when the next token merely looks similar.
        for plain in [
            "-C",
            "-C panic=abort",
            "-C opt-level=3 panic=immediate-abort-ish",
            "--codegen debuginfo=2",
        ] {
            assert!(!declares_immediate_abort(None, Some(plain)), "{plain}");
        }
    }

    /// `-Cpanic` is last-wins in rustc, so "the value appears somewhere" is
    /// the wrong question — only the last setting is in effect. A stream
    /// ending in `unwind` must not be reported as selecting immediate-abort
    /// just because it appears earlier, and vice versa.
    #[test]
    fn declares_immediate_abort_honors_the_last_setting_not_any_setting() {
        // Immediate-abort mentioned first, overridden by unwind: not in effect.
        assert!(!declares_immediate_abort(
            Some("-Cpanic=immediate-abort\u{1f}-Cpanic=unwind"),
            None
        ));
        assert!(!declares_immediate_abort(
            None,
            Some("-Zunstable-options -Cpanic=immediate-abort -Cpanic=unwind")
        ));
        // The reverse order: unwind first, immediate-abort wins.
        assert!(declares_immediate_abort(
            Some("-Cpanic=unwind\u{1f}-Cpanic=immediate-abort"),
            None
        ));
        // Mixed joined/split spellings, still last-wins.
        assert!(!declares_immediate_abort(
            None,
            Some("-C panic=immediate-abort -Cpanic=abort")
        ));
    }

    #[test]
    fn immediate_abort_implies_build_std() -> Result<()> {
        let inv = parse(&["cargo-no-alloc", "--immediate-abort", "--", "build"])?;
        assert!(inv.immediate_abort);
        // Rebuilding the sysroot is not optional here: the precompiled one
        // was built with a different panic strategy, and mixing them is
        // exactly what this flag exists to avoid.
        assert!(inv.build_std);
        // --build-std on its own must not turn the panic strategy on.
        let inv = parse(&["cargo-no-alloc", "--build-std", "--", "build"])?;
        assert!(inv.build_std);
        assert!(!inv.immediate_abort);
        Ok(())
    }

    #[test]
    fn immediate_abort_rejects_test_mode() {
        let error = parse(&["cargo-no-alloc", "--immediate-abort", "--", "test"])
            .expect_err("--immediate-abort -- test cannot build");
        // The point of rejecting at parse time is the message: rustc's own
        // failure arrives after a full sysroot rebuild and names an
        // unrelated-looking flag.
        assert!(error.to_string().contains("-- build"), "{error}");
        // Neither half is a problem on its own.
        assert!(parse(&["cargo-no-alloc", "--immediate-abort", "--", "build"]).is_ok());
        assert!(parse(&["cargo-no-alloc", "--", "test"]).is_ok());
    }

    #[test]
    fn immediate_abort_flag_is_appended_even_when_an_equal_flag_is_inherited() {
        // `-Cpanic` is last-wins, so "already present somewhere" is not the
        // same as "in effect": an inherited pair ending in `-Cpanic=unwind`
        // must not stop the checker's own copy from being appended last.
        let flags = |encoded: &str| {
            // SAFETY: see `orchestration_runs_with_fake_cargo_and_rustc` —
            // env mutation in this binary is serialized by `FAKE_RUSTC`.
            let _guard = FAKE_RUSTC
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let previous = std::env::var_os("CARGO_ENCODED_RUSTFLAGS");
            unsafe { std::env::set_var("CARGO_ENCODED_RUSTFLAGS", encoded) };
            let mut command = Command::new("true");
            add_required_rustflags(&mut command, true);
            let value = command
                .get_envs()
                .find(|(name, _)| *name == "CARGO_ENCODED_RUSTFLAGS")
                .and_then(|(_, value)| value)
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_default();
            unsafe {
                match previous {
                    Some(previous) => std::env::set_var("CARGO_ENCODED_RUSTFLAGS", previous),
                    None => std::env::remove_var("CARGO_ENCODED_RUSTFLAGS"),
                }
            }
            value
        };
        let outvoted = flags("-Cpanic=immediate-abort\u{1f}-Cpanic=unwind");
        let last = outvoted.split('\u{1f}').next_back().unwrap_or_default();
        assert_eq!(last, "-Cpanic=immediate-abort", "in `{outvoted}`");
    }

    #[test]
    fn aggregate_detects_immediate_abort_even_when_fragments_disagree() -> Result<()> {
        // The P1 scenario: a target-specific config selects immediate-abort
        // for the checked crate, while a wrapped host unit (a build script
        // with a checked root of its own) compiles under something else.
        // `Report::merge` correctly calls that disagreement `None` for
        // `report.json`'s own sake — but `aggregate`'s second return value
        // must still see the `ImmediateAbort` fragment, or the safety net in
        // `run` never fires.
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_aggregate_conflict_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&directory)?;
        let root = |name: &str| RootVerdict {
            root: name.to_owned(),
            instance: name.to_owned(),
            verdict: Verdict::Pass,
        };
        ReportFragment {
            report: Report {
                roots: vec![root("target::checked")],
                panic_strategy: Some(PanicStrategy::ImmediateAbort),
                ..Report::default()
            },
            matched_root_specs: vec![],
        }
        .write_to_file(&directory.join("target.json"))?;
        ReportFragment {
            report: Report {
                roots: vec![root("build_script::checked")],
                panic_strategy: Some(PanicStrategy::Unwind),
                ..Report::default()
            },
            matched_root_specs: vec![],
        }
        .write_to_file(&directory.join("host.json"))?;

        let (report, any_immediate_abort) = aggregate(&directory, &[])?;
        // The report itself is silent about the disagreement, as designed...
        assert_eq!(report.panic_strategy, None);
        // ...but the safety-net signal still catches the mix.
        assert!(any_immediate_abort);
        assert!(mixed_sysroot_panic_strategy(false, any_immediate_abort));

        std::fs::remove_dir_all(directory)?;
        Ok(())
    }

    #[test]
    fn mixed_sysroot_is_flagged_only_without_build_std() {
        // The dangerous case: crate compiled immediate-abort, sysroot left
        // precompiled (manifest/config selected the strategy, `--build-std`
        // was not passed).
        assert!(mixed_sysroot_panic_strategy(false, true));
        // `--build-std` rebuilds the sysroot under the same flags, so the
        // same observation is coherent once it is set.
        assert!(!mixed_sysroot_panic_strategy(true, true));
        // Nothing observed has nothing to be mixed with.
        assert!(!mixed_sysroot_panic_strategy(false, false));
    }

    #[test]
    fn immediate_abort_rejects_test_harness_target_selection() -> Result<()> {
        // `build --tests` reaches the same rustc failure as `-- test`, just
        // through target selection, and just as far into a sysroot rebuild.
        for selection in [vec!["--tests"], vec!["--all-targets"]] {
            let mut args = vec!["cargo-no-alloc", "--immediate-abort", "--", "build"];
            args.extend(selection.iter().copied());
            let error = parse(&args).expect_err(&format!("{selection:?} must be rejected"));
            assert!(error.to_string().contains("test-harness"), "{error}");
            // Only `--immediate-abort` has the problem; plain runs still build them.
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
            let mut args = vec!["cargo-no-alloc", "--immediate-abort", "--", "build"];
            args.extend(selection.iter().copied());
            assert!(parse(&args).is_ok(), "{selection:?} must be allowed");
        }
        // Target selections that build no harness stay accepted.
        assert!(parse(&[
            "cargo-no-alloc",
            "--immediate-abort",
            "--",
            "build",
            "--lib"
        ])
        .is_ok());
        assert!(parse(&[
            "cargo-no-alloc",
            "--immediate-abort",
            "--",
            "build",
            "--examples"
        ])
        .is_ok());
        // Arguments after the test-runner separator are not target selection.
        parse(&[
            "cargo-no-alloc",
            "--immediate-abort",
            "--",
            "build",
            "--",
            "--tests",
        ])?;
        Ok(())
    }

    #[test]
    fn immediate_abort_rustflags_are_added_only_when_asked_for() {
        // `add_required_rustflags` reads the ambient RUSTFLAGS/
        // CARGO_ENCODED_RUSTFLAGS directly, so this needs the same isolation
        // `immediate_abort_flag_is_appended_even_when_an_equal_flag_is_inherited`
        // does: without it, either that test's transient mutation or a
        // developer's own shell exporting `-Cpanic=immediate-abort` could
        // leak in and fail the `without` assertions spuriously.
        let _guard = FAKE_RUSTC
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let saved = [
            ("RUSTFLAGS", std::env::var_os("RUSTFLAGS")),
            (
                "CARGO_ENCODED_RUSTFLAGS",
                std::env::var_os("CARGO_ENCODED_RUSTFLAGS"),
            ),
        ];
        // SAFETY: guarded by `FAKE_RUSTC`, restored before returning.
        unsafe {
            std::env::remove_var("RUSTFLAGS");
            std::env::remove_var("CARGO_ENCODED_RUSTFLAGS");
        }
        let flags = |immediate_abort| {
            let mut command = Command::new("true");
            add_required_rustflags(&mut command, immediate_abort);
            command
                .get_envs()
                .find(|(name, _)| *name == "RUSTFLAGS" || *name == "CARGO_ENCODED_RUSTFLAGS")
                .and_then(|(_, value)| value)
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_default()
        };
        let without = flags(false);
        let with = flags(true);
        // SAFETY: same guard as above.
        unsafe {
            for (name, value) in saved {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
        for flag in IMMEDIATE_ABORT_RUSTFLAGS {
            assert!(!without.contains(flag), "unasked-for `{flag}`");
            assert!(with.contains(flag), "missing `{flag}`");
        }
        for flag in REQUIRED_RUSTFLAGS {
            assert!(without.contains(flag), "missing `{flag}`");
            assert!(with.contains(flag), "missing `{flag}`");
        }
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
        assert!(invocation.build_std);
        assert!(invocation.all_crates);
        assert_eq!(invocation.roots, ["z::f"]);
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
            "--immediate-abort",
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

    #[test]
    fn run_prints_help_and_version_without_touching_toolchain_or_cargo() {
        // Deliberately does not set RUSTC/CARGO to a fake: if either flag's
        // handling fell through to the real orchestration path, this would
        // fail by trying (and likely failing) to run the real `cargo`/`rustc`.
        assert!(run(["cargo-no-alloc".to_owned(), "--help".to_owned()]).is_ok());
        assert!(run(["cargo-no-alloc".to_owned(), "--version".to_owned()]).is_ok());
    }

    #[test]
    fn zero_roots_warning_names_the_two_likely_causes() {
        assert!(ZERO_ROOTS_WARNING.contains("--all-crates"));
        assert!(ZERO_ROOTS_WARNING.contains("0 root instances"));
    }

    #[test]
    fn verify_pinned_toolchain_accepts_the_pinned_version() -> Result<()> {
        let _guard = FAKE_RUSTC
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_toolchain_ok_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&directory)?;
        let fake_rustc = directory.join("rustc");
        std::fs::write(
            &fake_rustc,
            "#!/bin/sh\nprintf 'release: 1.99.0-nightly\\ncommit-hash: ad3d0bc14\\nhost: x86_64-unknown-linux-gnu\\n'\n",
        )?;
        let mut permissions = std::fs::metadata(&fake_rustc)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake_rustc, permissions)?;

        let result = verify_pinned_toolchain(&fake_rustc.to_string_lossy());
        std::fs::remove_dir_all(directory)?;
        result
    }

    #[test]
    fn verify_pinned_toolchain_rejects_a_mismatched_version() -> Result<()> {
        let _guard = FAKE_RUSTC
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_toolchain_bad_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&directory)?;
        let fake_rustc = directory.join("rustc");
        std::fs::write(
            &fake_rustc,
            "#!/bin/sh\nprintf 'release: 1.98.0-stable\\ncommit-hash: deadbeef00\\nhost: x86_64-unknown-linux-gnu\\n'\n",
        )?;
        let mut permissions = std::fs::metadata(&fake_rustc)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake_rustc, permissions)?;

        let error = verify_pinned_toolchain(&fake_rustc.to_string_lossy())
            .expect_err("mismatched toolchain must be rejected");
        assert!(error.to_string().contains("nightly-2026-08-01"));
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }

    #[test]
    fn aggregate_ignores_non_json_and_reports_unmatched() -> Result<()> {
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_aggregate_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&directory)?;
        std::fs::write(directory.join("ignore.txt"), "not json")?;
        ReportFragment {
            report: Report::default(),
            matched_root_specs: vec!["matched::root".into()],
        }
        .write_to_file(&directory.join("fragment.json"))?;

        let (report, any_immediate_abort) = aggregate(
            &directory,
            &["matched::root".into(), "missing::root".into()],
        )?;
        assert_eq!(report.selection_errors.len(), 1);
        assert!(report.selection_errors[0].contains("missing::root"));
        assert!(!any_immediate_abort);
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }

    #[test]
    fn aggregate_accepts_missing_fragment_directory() -> Result<()> {
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_missing_{}_{}",
            std::process::id(),
            line!()
        ));
        let (report, any_immediate_abort) = aggregate(&directory, &[])?;
        assert_eq!(report, Report::default());
        assert!(!any_immediate_abort);
        Ok(())
    }

    #[test]
    fn orchestration_runs_with_fake_cargo_and_rustc() -> Result<()> {
        // Also writes executables and runs them, so it shares the lock (see
        // `FAKE_RUSTC`) as well as being the only test that mutates the
        // process environment.
        let _guard = FAKE_RUSTC
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let directory = std::env::temp_dir().join(format!(
            "cargo_no_alloc_orchestration_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&directory)?;
        let fake_rustc = directory.join("rustc");
        std::fs::write(
            &fake_rustc,
            // `run` now checks `-vV` (verify_pinned_toolchain) before it
            // ever reaches the `--print host-tuple`/`--print sysroot` calls
            // this script already answered.
            "#!/bin/sh\nif [ \"$1\" = \"-vV\" ]; then printf 'release: 1.99.0-nightly\\ncommit-hash: ad3d0bc14\\nhost: x86_64-unknown-linux-gnu\\n'; exit 0; fi\nif [ \"$1 $2\" = \"--print host-tuple\" ]; then echo x86_64-unknown-linux-gnu; exit 0; fi\nif [ \"$1 $2\" = \"--print sysroot\" ]; then echo /fake/sysroot; exit 0; fi\nexit 1\n",
        )?;
        let fake_cargo = directory.join("cargo");
        std::fs::write(
            &fake_cargo,
            "#!/bin/sh\nif [ \"$1\" = clean ]; then exit 0; fi\nmkdir -p \"$NO_ALLOC_FRAGMENT_DIR\"\nprintf '%s\\n' '{\"report\":{\"roots\":[]},\"matched_root_specs\":[\"fake::root\"]}' > \"$NO_ALLOC_FRAGMENT_DIR/fake.json\"\nif [ \"$FAKE_CARGO_FAIL\" = 1 ]; then exit 1; fi\n",
        )?;
        for executable in [&fake_rustc, &fake_cargo] {
            let mut permissions = std::fs::metadata(executable)?.permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(executable, permissions)?;
        }

        let original_directory = std::env::current_dir()?;
        let variables = [
            "CARGO",
            "RUSTC",
            "CARGO_ENCODED_RUSTFLAGS",
            "NO_ALLOC_WARN_ONLY",
            "NO_ALLOC_ROOTS",
            "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER",
            "FAKE_CARGO_FAIL",
        ];
        let originals: Vec<_> = variables
            .iter()
            .map(|name| ((*name).to_owned(), std::env::var_os(name)))
            .collect();

        // SAFETY: this is the only test in this process that mutates the
        // process environment/current directory; the other tests are pure.
        unsafe {
            std::env::set_var("CARGO", &fake_cargo);
            std::env::set_var("RUSTC", &fake_rustc);
            std::env::set_var("CARGO_ENCODED_RUSTFLAGS", "--cfg=no_alloc_check");
            std::env::set_var("NO_ALLOC_WARN_ONLY", "1");
            std::env::set_var("NO_ALLOC_ROOTS", "fake::root");
            std::env::remove_var("RUSTC_WRAPPER");
            std::env::remove_var("RUSTC_WORKSPACE_WRAPPER");
        }
        std::env::set_current_dir(&directory)?;

        let result = run([
            "cargo-no-alloc".to_owned(),
            "--all-crates".to_owned(),
            "--build-std".to_owned(),
            "--root=fake::root".to_owned(),
            "--".to_owned(),
            "test".to_owned(),
            "--".to_owned(),
            "--list".to_owned(),
        ]);

        std::env::set_current_dir(original_directory)?;
        for (name, value) in originals {
            // SAFETY: restoration occurs before this test returns, under the
            // same single-mutator condition described above.
            unsafe {
                if let Some(value) = value {
                    std::env::set_var(name, value);
                } else {
                    std::env::remove_var(name);
                }
            }
        }
        std::fs::remove_dir_all(directory)?;
        result
    }
}
