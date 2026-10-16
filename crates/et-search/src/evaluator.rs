use et_engine::{Action, GameState, Observation};
use rand::Rng;

/// Prior over legal actions plus a value estimate in [-1, 1] from the
/// perspective of the player to move.
#[derive(Debug, Clone)]
pub struct Evaluation {
    pub priors: Vec<(Action, f32)>,
    pub value: f32,
}

/// Anything that can score a leaf. Implemented by random rollouts in this
/// crate and by the gRPC-backed network client in `et-server`.
pub trait Evaluator: Send + Sync {
    fn evaluate(&self, obs: &Observation, legal: &[Action]) -> Evaluation;
}

/// Baseline evaluator: uniform priors and a value from random playout.
pub struct RolloutEvaluator {
    pub max_depth: u32,
}

impl Evaluator for RolloutEvaluator {
    fn evaluate(&self, obs: &Observation, legal: &[Action]) -> Evaluation {
        let p = 1.0 / legal.len().max(1) as f32;
        Evaluation {
            priors: legal.iter().map(|a| (a.clone(), p)).collect(),
            value: rollout_value(obs, self.max_depth),
        }
    }
}

/// Placeholder until the engine can determinize an observation into a state.
fn rollout_value(_obs: &Observation, _max_depth: u32) -> f32 {
    let mut rng = rand::rng();
    let _state = GameState::new();
    rng.random_range(-1.0..=1.0)
}
