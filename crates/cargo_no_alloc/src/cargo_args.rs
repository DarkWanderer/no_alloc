//! Scanning the Cargo arguments the checker forwards. The scanners
//! deliberately stay as dumb as they were: an option's value is only consumed
//! for the options asked about, so a look-alike value of another option is
//! still seen as an argument.

/// Splits at the first bare `--`: Cargo's own arguments before it, the test
/// runner's after it (separator included). Anything past the separator is not
/// Cargo's to interpret, so every scanner works on the first half only.
pub(crate) fn split_at_separator(args: &[String]) -> (&[String], &[String]) {
    let split = args
        .iter()
        .position(|arg| arg == "--")
        .unwrap_or(args.len());
    args.split_at(split)
}

/// How an argument spells a value-taking option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptionForm {
    /// `--name=value`: one token.
    Joined,
    /// `--name`, with the value in the next token.
    Separate,
}

/// Whether `arg` is one of the value-taking options `names`, and in which
/// spelling. Exact on the name, so `--profiler` is not `--profile`.
pub(crate) fn valued_option(arg: &str, names: &[&str]) -> Option<OptionForm> {
    names.iter().find_map(|name| {
        if arg == *name {
            Some(OptionForm::Separate)
        } else if arg
            .strip_prefix(name)
            .is_some_and(|rest| rest.starts_with('='))
        {
            Some(OptionForm::Joined)
        } else {
            None
        }
    })
}

/// The tokens, verbatim and in order, of the bare `flags` and the value-taking
/// `valued` options among Cargo's own arguments. The one scanner behind every
/// "which of the build's options does this other Cargo invocation need".
pub(crate) fn forward_options(args: &[String], flags: &[&str], valued: &[&str]) -> Vec<String> {
    let mut forwarded = Vec::new();
    let mut options = split_at_separator(args).0.iter();
    while let Some(arg) = options.next() {
        if flags.contains(&arg.as_str()) {
            forwarded.push(arg.clone());
            continue;
        }
        match valued_option(arg, valued) {
            Some(OptionForm::Separate) => {
                forwarded.push(arg.clone());
                forwarded.extend(options.next().cloned());
            }
            Some(OptionForm::Joined) => forwarded.push(arg.clone()),
            None => {}
        }
    }
    forwarded
}

/// The build's own options that decide *which* artifacts `cargo clean`
/// must remove. Without them `cargo clean --workspace` clears only the dev
/// profile of the workspace in the current directory, so a `--release` or
/// `--profile` build (or one of another `--manifest-path`) found its
/// workspace units fresh, the driver never ran on them, and their roots
/// silently dropped out of a report that still read as a success.
pub(crate) fn clean_selection_args(cargo_args: &[String]) -> Vec<String> {
    forward_options(
        cargo_args,
        &["--release", "-r"],
        &["--profile", "--manifest-path", "--config"],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::strings;

    #[test]
    fn clean_selection_args_forwards_profile_and_workspace_selection() {
        assert_eq!(
            clean_selection_args(&strings(&[
                "--release",
                "-p",
                "a",
                "--profile",
                "custom",
                "--manifest-path=other/Cargo.toml",
                "--config",
                "profile.custom.inherits='release'",
                "--features",
                "x",
            ])),
            strings(&[
                "--release",
                "--profile",
                "custom",
                "--manifest-path=other/Cargo.toml",
                "--config",
                "profile.custom.inherits='release'",
            ])
        );
        assert_eq!(
            clean_selection_args(&strings(&["-r", "--profile=bench", "--manifest-path", "m"])),
            strings(&["-r", "--profile=bench", "--manifest-path", "m"])
        );
    }

    #[test]
    fn clean_selection_args_stops_at_the_test_runner_separator() {
        assert_eq!(
            clean_selection_args(&strings(&["--lib", "--", "--release", "--profile", "x"])),
            Vec::<String>::new()
        );
        // A look-alike flag is not a profile selection.
        assert!(clean_selection_args(&strings(&["--profiler", "--releases"])).is_empty());
    }

    #[test]
    fn split_at_separator_keeps_the_separator_with_the_runner_half() {
        let args = strings(&["build", "--", "--nocapture", "--"]);
        let (cargo, runner) = split_at_separator(&args);
        assert_eq!(cargo, strings(&["build"]));
        assert_eq!(runner, strings(&["--", "--nocapture", "--"]));
        let args = strings(&["a", "b"]);
        let (cargo, runner) = split_at_separator(&args);
        assert_eq!((cargo.len(), runner.len()), (2, 0));
    }

    #[test]
    fn valued_option_is_exact_on_the_name() {
        assert_eq!(
            valued_option("--config", &["--config"]),
            Some(OptionForm::Separate)
        );
        assert_eq!(
            valued_option("--config=a=1", &["--config"]),
            Some(OptionForm::Joined)
        );
        assert_eq!(valued_option("--configs", &["--config"]), None);
        assert_eq!(valued_option("--config", &["--manifest-path"]), None);
    }

    /// The pinned behaviour of not being smarter than the original scanners: a
    /// value of an option the caller did not ask about is still scanned as an
    /// argument, while the value of one it did ask about is consumed.
    #[test]
    fn values_of_unrelated_options_are_not_skipped() {
        let args = strings(&["--features", "--release", "--config", "--release"]);
        assert_eq!(
            forward_options(&args, &["--release"], &["--config"]),
            strings(&["--release", "--config", "--release"])
        );
    }
}
