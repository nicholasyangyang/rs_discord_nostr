use std::path::PathBuf;

use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt};

mod error;
mod transport;
mod config;
mod keys;
mod state;
mod nostr;
mod discord;
mod app;

/// Discord ↔ Nostr 消息桥
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// 数据目录（存放 key.json）
    #[arg(long, value_name = "DIR")]
    cwd_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let _ = dotenvy::dotenv();

    fmt()
        .with_env_filter(
            EnvFilter::try_from_env("LOG_LEVEL")
                .unwrap_or_else(|_| EnvFilter::from_default_env()),
        )
        .init();

    std::fs::create_dir_all(&cli.cwd_dir)?;

    app::run(cli.cwd_dir).await
}
