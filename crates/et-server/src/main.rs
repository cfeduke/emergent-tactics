//! Game server. Owns sessions, applies rules via `et-engine`, and asks
//! `et-search` for AI moves. Talks JSON over WebSocket to the web client and
//! (later) gRPC to the inference service.

mod grpc_eval;
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
    /// Pause after each AI move so the client can show the play.
    #[arg(long, default_value_t = 700)]
    ai_delay_ms: u64,
}

#[derive(Clone)]
pub struct AppState {
    pub ai_delay_ms: u64,
    /// Connected inference client, if the service was reachable and its
    /// model matches the engine's library.
    pub inference: Option<et_protocol::inference::inference_client::InferenceClient<tonic::transport::Channel>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let args = Args::parse();
    let inference = check_inference(&args.inference).await?;
    let state = Arc::new(AppState { ai_delay_ms: args.ai_delay_ms, inference });
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

/// Refuse to start against an inference service whose model was trained
/// on a different card library. An unreachable service is only a warning:
/// rollout evaluation still works without it.
async fn check_inference(endpoint: &str) -> anyhow::Result<Option<et_protocol::inference::inference_client::InferenceClient<tonic::transport::Channel>>> {
    use et_protocol::inference::{InfoRequest, inference_client::InferenceClient};
    let ours = et_engine::library_fingerprint_hex();
    let connect = tokio::time::timeout(std::time::Duration::from_secs(2), InferenceClient::connect(endpoint.to_string()));
    let mut client = match connect.await {
        Ok(Ok(c)) => c,
        Ok(Err(e)) => {
            tracing::warn!(endpoint, error = %e, "inference service unreachable; net-* presets fall back to rollouts");
            return Ok(None);
        }
        Err(_) => {
            tracing::warn!(endpoint, "inference service connect timed out; net-* presets fall back to rollouts");
            return Ok(None);
        }
    };
    let info = client.info(InfoRequest {}).await?.into_inner();
    if !info.library_fingerprint.is_empty() && info.library_fingerprint != ours {
        anyhow::bail!("inference model fingerprint {} does not match engine library {ours}", info.library_fingerprint);
    }
    tracing::info!(checkpoint = %info.checkpoint, fingerprint = %info.library_fingerprint, engine = %ours, "inference service ok");
    Ok(Some(client))
}
