use anyhow::{bail, ensure, Context, Result};
use fs2::FileExt;
use no_alloc_report::{parse_root_spec, Report, ReportFragment, Verdict};
use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

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

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Invocation> {
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
    Ok(Invocation {
        build_std,
        all_crates,
        warn_only,
        roots,
        command,
        cargo_args: cargo,
    })
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
    let mut inv = parse_args(args)?;
    if env::var("NO_ALLOC_WARN_ONLY").as_deref() == Ok("1") {
        inv.warn_only = true;
    }
    if env::var_os("RUSTC_WRAPPER").is_some() || env::var_os("RUSTC_WORKSPACE_WRAPPER").is_some() {
        bail!("existing RUSTC_WRAPPER/RUSTC_WORKSPACE_WRAPPER configuration is incompatible with cargo-no-alloc");
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

    fn parse(args: &[&str]) -> Result<Invocation> {
        parse_args(args.iter().map(|arg| (*arg).to_owned()))
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
            "#!/bin/sh\nif [ \"$1 $2\" = \"--print host-tuple\" ]; then echo x86_64-unknown-linux-gnu; exit 0; fi\nexit 1\n",
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
