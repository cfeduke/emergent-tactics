//! Evaluator that asks the inference service over gRPC, one leaf at a time.
//! Used from the search's blocking thread via the runtime handle.

use et_engine::{Action, Observation, encode};
use et_protocol::inference::{EvaluateRequest, inference_client::InferenceClient};
use et_search::{Evaluation, Evaluator, priors_from_logits};
use tonic::transport::Channel;

pub struct GrpcEvaluator {
    client: InferenceClient<Channel>,
    handle: tokio::runtime::Handle,
}

impl GrpcEvaluator {
    pub fn new(client: InferenceClient<Channel>, handle: tokio::runtime::Handle) -> Self {
        Self { client, handle }
    }
}

impl Evaluator for GrpcEvaluator {
    fn evaluate(&self, obs: &Observation, legal: &[Action], _rng: &mut dyn rand::RngCore) -> Evaluation {
        let features = encode::encode_observation(obs);
        let req = EvaluateRequest { features, batch_size: 1, feature_len: encode::FEATURE_LEN as u32, legal_mask: Vec::new() };
        let mut client = self.client.clone();
        let resp = self.handle.block_on(async move { client.evaluate(req).await });
        match resp {
            Ok(r) => {
                let r = r.into_inner();
                let value = r.value.first().copied().unwrap_or(0.0).clamp(-1.0, 1.0);
                Evaluation { priors: priors_from_logits(&r.policy, legal), value }
            }
            Err(e) => {
                tracing::warn!(error = %e, "inference call failed; uniform priors");
                let p = 1.0 / legal.len().max(1) as f32;
                Evaluation { priors: legal.iter().map(|a| (*a, p)).collect(), value: 0.0 }
            }
        }
    }
}
