// Never compiled: ambient flags selecting `-Cpanic=immediate-abort` without
// `--build-std` are refused before the build. The subject of this case is the
// persisted refusal.
fn main() {}
