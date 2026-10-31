//! Shared by every module's unit tests.

/// Serializes every test that writes a fake `rustc` and executes it, or that
/// mutates the process environment or current directory.
///
/// One static for the whole crate, on purpose: a lock per module would let
/// two modules' tests run each other's critical sections, which brings back
/// the intermittent failures this exists to prevent.
///
/// Writing an executable and running it is not thread-safe against a
/// *sibling* doing the same: the other test's `Command::spawn` inherits
/// the still-open write fd, and the exec then fails with `ETXTBSY`
/// ("Text file busy"). It reproduced about half the time here, as
/// whichever test lost the race failing with a spawn error instead of
/// the verdict it asserts on.
pub(crate) static FAKE_RUSTC: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_owned()).collect()
}
