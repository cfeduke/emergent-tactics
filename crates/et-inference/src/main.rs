//! Inference service. Game-agnostic: receives feature batches over gRPC and
//! returns policy logits and values. ONNX loading via `ort` is added once a
//! first model has been exported.

use clap::Parser;
use et_protocol::inference::inference_server::{Inference, InferenceServer};
use et_protocol::inference::{EvaluateRequest, EvaluateResponse, InfoRequest, InfoResponse};
use tonic::{Request, Response, Status, transport::Server};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:50051")]
    listen: String,
    #[arg(long)]
    model: Option<std::path::PathBuf>,
    /// Card-library fingerprint the model was trained against. Will be
    /// read from the ONNX metadata once models exist.
    #[arg(long, default_value = "")]
    library_fingerprint: String,
}

struct Service {
    checkpoint: String,
    library_fingerprint: String,
}

#[tonic::async_trait]
impl Inference for Service {
    async fn evaluate(&self, req: Request<EvaluateRequest>) -> Result<Response<EvaluateResponse>, Status> {
        let r = req.into_inner();
        let n = r.batch_size as usize;
        // Placeholder: uniform policy, neutral value.
        Ok(Response::new(EvaluateResponse {
            policy: vec![0.0; n * et_engine_action_space()],
            value: vec![0.0; n],
        }))
    }

    async fn info(&self, _req: Request<InfoRequest>) -> Result<Response<InfoResponse>, Status> {
        Ok(Response::new(InfoResponse {
            model_name: "placeholder".into(),
            checkpoint: self.checkpoint.clone(),
            feature_len: 0,
            action_space: et_engine_action_space() as u32,
            library_fingerprint: self.library_fingerprint.clone(),
        }))
    }
}

fn et_engine_action_space() -> usize {
    1
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let args = Args::parse();
    let addr = args.listen.parse()?;
    let checkpoint = args.model.map(|p| p.display().to_string()).unwrap_or_else(|| "none".into());
    tracing::info!(%addr, %checkpoint, fingerprint = %args.library_fingerprint, "inference service listening");
    let service = Service { checkpoint, library_fingerprint: args.library_fingerprint };
    Server::builder().add_service(InferenceServer::new(service)).serve(addr).await?;
    Ok(())
}
