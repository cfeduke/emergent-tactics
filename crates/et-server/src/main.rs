//! Game server. Owns sessions, applies rules via `et-engine`, and asks
//! `et-search` for AI moves. Talks JSON over WebSocket to the web client and
//! (later) gRPC to the inference service.

mod session;

use axum::{
    Router,
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::Response,
    routing::get,
};
use clap::Parser;
use std::sync::Arc;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:8080")]
    listen: String,
    #[arg(long, default_value = "http://127.0.0.1:50051")]
    inference: String,
    /// Pause between AI moves in spectated games, so they are watchable.
    #[arg(long, default_value_t = 700)]
    spectate_delay_ms: u64,
}

#[derive(Clone)]
pub struct AppState {
    pub spectate_delay_ms: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let args = Args::parse();
    let state = Arc::new(AppState { spectate_delay_ms: args.spectate_delay_ms });
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(ws_handler))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(&args.listen).await?;
    tracing::info!(listen = %args.listen, inference = %args.inference, "game server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(move |socket: WebSocket| session::run(socket, state))
}
