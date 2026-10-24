fn main() -> anyhow::Result<()> {
    // Same filter variable and default as the driver, so one `NO_ALLOC_LOG`
    // setting controls both halves of a run.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("NO_ALLOC_LOG")
                .unwrap_or_else(|_| "off".into()),
        )
        .init();
    cargo_no_alloc::run(std::env::args())
}
