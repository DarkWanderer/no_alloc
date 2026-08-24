use anyhow::{bail, ensure, Context, Result};
use fs2::FileExt;
use no_alloc_report::{parse_root_spec, Report, ReportFragment, Verdict};
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
    }
    roots.sort();
    roots.dedup();
    Ok(ParsedArgs::Run(Invocation {
        build_std,
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

fn add_required_rustflags(command: &mut Command) {
    if let Some(encoded) = env::var_os("CARGO_ENCODED_RUSTFLAGS") {
        let mut value = encoded.to_string_lossy().into_owned();
        let existing: HashSet<String> = value.split('\x1f').map(str::to_owned).collect();
        for flag in REQUIRED_RUSTFLAGS {
            if !existing.contains(*flag) {
                if !value.is_empty() {
                    value.push('\x1f');
                }
                value.push_str(flag);
            }
        }
        command.env("CARGO_ENCODED_RUSTFLAGS", value);
    } else {
        let mut value = env::var("RUSTFLAGS").unwrap_or_default();
        for flag in REQUIRED_RUSTFLAGS {
            if !value.split_whitespace().any(|existing| existing == *flag) {
                if !value.is_empty() {
                    value.push(' ');
                }
                value.push_str(flag);
            }
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

fn aggregate(fragment_dir: &Path, requested: &[String]) -> Result<Report> {
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
    Ok(report)
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
    add_required_rustflags(&mut command);
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
    let report = aggregate(&fragment_dir, &inv.roots)?;
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
    use super::*;
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

        let report = aggregate(
            &directory,
            &["matched::root".into(), "missing::root".into()],
        )?;
        assert_eq!(report.selection_errors.len(), 1);
        assert!(report.selection_errors[0].contains("missing::root"));
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
        let report = aggregate(&directory, &[])?;
        assert_eq!(report, Report::default());
        Ok(())
    }

    #[test]
    fn orchestration_runs_with_fake_cargo_and_rustc() -> Result<()> {
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
