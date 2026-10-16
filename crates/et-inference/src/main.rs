//! Inference service. Game-agnostic: receives feature batches over gRPC and
//! returns policy logits and values from an ONNX policy/value model.

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use et_model::OnnxModel;
use et_protocol::inference::inference_server::{Inference, InferenceServer};
use et_protocol::inference::{EvaluateRequest, EvaluateResponse, InfoRequest, InfoResponse};
use tonic::{Request, Response, Status, transport::Server};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:50051")]
    listen: String,
    /// ONNX model exported by `et-export`.
    #[arg(long)]
    model: PathBuf,
    /// ONNX Runtime intra-op threads per evaluation.
    #[arg(long, default_value_t = 2)]
    intra_threads: usize,
}

struct Service {
    model: Arc<OnnxModel>,
    checkpoint: String,
}

#[tonic::async_trait]
impl Inference for Service {
    async fn evaluate(&self, req: Request<EvaluateRequest>) -> Result<Response<EvaluateResponse>, Status> {
        let r = req.into_inner();
        let n = r.batch_size as usize;
        let info = self.model.info();
        if r.feature_len as usize != info.feature_len || r.features.len() != n * info.feature_len {
            return Err(Status::invalid_argument(format!("expected {} features per row, {} rows", info.feature_len, n)));
        }
        let model = self.model.clone();
        let (policy, value) = tokio::task::spawn_blocking(move || model.evaluate(&r.features, n))
            .await
            .map_err(|e| Status::internal(e.to_string()))?
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(EvaluateResponse { policy, value }))
    }

    async fn info(&self, _req: Request<InfoRequest>) -> Result<Response<InfoResponse>, Status> {
        let i = self.model.info();
        Ok(Response::new(InfoResponse {
            model_name: i.name.clone(),
            checkpoint: self.checkpoint.clone(),
            feature_len: i.feature_len as u32,
            action_space: i.action_space as u32,
            library_fingerprint: i.library_fingerprint.clone(),
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    let args = Args::parse();
    let addr = args.listen.parse()?;
    let model = Arc::new(OnnxModel::load(&args.model, args.intra_threads)?);
    let info = model.info().clone();
    tracing::info!(%addr, model = %args.model.display(), name = %info.name, fingerprint = %info.library_fingerprint, feature_len = info.feature_len, action_space = info.action_space, "inference service listening");
    let service = Service { model, checkpoint: args.model.display().to_string() };
    Server::builder().add_service(InferenceServer::new(service)).serve(addr).await?;
    Ok(())
}
