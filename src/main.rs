use anyhow::Context;
use clap::Parser;
use std::io;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::cli::Config;
use crate::server::build_app;

mod banner;
mod cli;
mod handlers;
mod html;
mod logging;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse arguments
    let config = Config::parse();

    let root = std::fs::canonicalize(&config.path)
        .with_context(|| format!("cannot serve {}", config.path.display()))?;
    anyhow::ensure!(
        root.is_dir(),
        "cannot serve {}: not a directory",
        config.path.display()
    );

    // Initialize logging (for libraries mainly, as we have custom request logging)
    if !config.silent {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "zserv=warn,info".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    // Bind before printing the banner, so it never advertises a port we failed to get
    // and shows the real one when `--port 0` asked the OS to pick.
    let requested = SocketAddr::from((config.address, config.port));
    let listener = tokio::net::TcpListener::bind(requested)
        .await
        .map_err(|err| match err.kind() {
            io::ErrorKind::AddrInUse => {
                anyhow::anyhow!("{requested} is already in use; pick another port with --port")
            }
            _ => anyhow::Error::new(err).context(format!("cannot listen on {requested}")),
        })?;
    let addr = listener.local_addr()?;

    if !config.silent {
        banner::print_banner(&config, &root, addr);
    }

    axum::serve(listener, build_app(&config, root)).await?;

    Ok(())
}
