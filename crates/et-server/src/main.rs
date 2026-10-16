//! Game server. Owns sessions, applies rules via `et-engine`, and asks
//! `et-search` for AI moves. Talks JSON over WebSocket to the web client and
//! gRPC to the inference service.

use axum::{Router, routing::get};
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: String,
    #[arg(long, default_value = "http://127.0.0.1:50051")]
    inference: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let args = Args::parse();
    let app = Router::new().route("/health", get(|| async { "ok" }));
    let listener = tokio::net::TcpListener::bind(&args.listen).await?;
    tracing::info!(listen = %args.listen, inference = %args.inference, "game server listening");
    axum::serve(listener, app).await?;
    Ok(())
}
