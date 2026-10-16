//! Network-backed evaluator: encodes the observation, asks the batcher for
//! logits and a value, and turns the logits into priors over legal actions.

use et_engine::{Action, Observation, encode};
use et_model::Batcher;

use crate::evaluator::{Evaluation, Evaluator};

pub struct NetworkEvaluator {
    batcher: Batcher,
}

impl NetworkEvaluator {
    pub fn new(batcher: Batcher) -> anyhow::Result<Self> {
        anyhow::ensure!(
            batcher.feature_len() == encode::FEATURE_LEN && batcher.action_space() == encode::ACTION_SPACE,
            "model dims {}x{} do not match engine {}x{}",
            batcher.feature_len(),
            batcher.action_space(),
            encode::FEATURE_LEN,
            encode::ACTION_SPACE
        );
        Ok(Self { batcher })
    }
}

/// Softmax over the logits of the legal actions only.
pub fn priors_from_logits(logits: &[f32], legal: &[Action]) -> Vec<(Action, f32)> {
    let picked: Vec<(Action, f32)> = legal.iter().map(|a| (*a, logits[encode::action_index(a)])).collect();
    let max = picked.iter().map(|(_, l)| *l).fold(f32::NEG_INFINITY, f32::max);
    let exps: Vec<f32> = picked.iter().map(|(_, l)| (l - max).exp()).collect();
    let sum: f32 = exps.iter().sum();
    picked.into_iter().zip(exps).map(|((a, _), e)| (a, if sum > 0.0 { e / sum } else { 1.0 / legal.len().max(1) as f32 })).collect()
}

impl Evaluator for NetworkEvaluator {
    fn evaluate(&self, obs: &Observation, legal: &[Action], _rng: &mut dyn rand::RngCore) -> Evaluation {
        let (logits, value) = self.batcher.evaluate(encode::encode_observation(obs));
        Evaluation { priors: priors_from_logits(&logits, legal), value: value.clamp(-1.0, 1.0) }
    }
}
