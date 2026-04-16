use anyhow::{Context, Result};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize tracing: file-based logging to ~/.local/state/flowbit/flowbit.log.
/// Returns a guard that must be held for the duration of the program.
pub fn init() -> Result<WorkerGuard> {
    let state_dir = if let Ok(xdg) = std::env::var("XDG_STATE_HOME") {
        std::path::PathBuf::from(xdg)
    } else {
        dirs::home_dir()
            .context("Could not determine home directory")?
            .join(".local")
            .join("state")
    };
    let log_dir = state_dir.join("flowbit");
    std::fs::create_dir_all(&log_dir)
        .with_context(|| format!("Failed to create log directory: {}", log_dir.display()))?;

    let file_appender = tracing_appender::rolling::never(&log_dir, "flowbit.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("flowbit=info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(true)
                .with_thread_ids(false),
        )
        .init();

    Ok(guard)
}
