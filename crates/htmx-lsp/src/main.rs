#![forbid(unsafe_code)]

use clap::Parser;
use htmx_analyzer::{ExitKind, Server};
use lsp_server::Connection;
use std::fs::OpenOptions;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// A fast, production-grade HTMX 2.0 language server and static analyzer
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Listen on stdio (default)
    #[arg(long, default_value_t = true)]
    stdio: bool,

    /// Optional path to log file
    #[arg(long)]
    file: Option<PathBuf>,

    /// Enable verbose debug logging
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

    let default_filter = if args.verbose { "debug" } else { "info" };
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

    if let Some(log_path) = args.file {
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        let file_layer = fmt::layer().with_writer(file).with_ansi(false);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .init();
        info!("Logging initialized to file: {:?}", log_path);
    } else {
        let stderr_layer = fmt::layer().with_writer(std::io::stderr).with_ansi(false);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(stderr_layer)
            .init();
    }

    info!(
        "Starting htmx-analyzer (version {})",
        env!("CARGO_PKG_VERSION")
    );

    let (connection, io_threads) = Connection::stdio();
    let server = Server::new(connection);

    let exit_kind = server.run()?;
    io_threads.join()?;

    info!("htmx-analyzer shutdown complete.");
    if exit_kind == ExitKind::NoShutdown {
        std::process::exit(1);
    }
    Ok(())
}
