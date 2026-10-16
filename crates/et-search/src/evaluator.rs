use et_engine::{Action, Observation, Outcome, Player};
use rand::seq::IndexedRandom;

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
    fn evaluate(&self, obs: &Observation, legal: &[Action], rng: &mut dyn rand::RngCore) -> Evaluation;
}

/// Baseline evaluator: uniform priors and a value from a uniformly random
/// playout of a state sampled from the observation.
pub struct RolloutEvaluator {
    pub max_depth: u32,
}

impl Evaluator for RolloutEvaluator {
    fn evaluate(&self, obs: &Observation, legal: &[Action], rng: &mut dyn rand::RngCore) -> Evaluation {
        let p = 1.0 / legal.len().max(1) as f32;
        Evaluation {
            priors: legal.iter().map(|a| (*a, p)).collect(),
            value: rollout_value(obs, self.max_depth, rng),
        }
    }
}

/// Value of a terminal outcome for `player`.
pub fn outcome_value(outcome: Outcome, player: Player) -> f32 {
    match outcome {
        Outcome::Win(w) if w == player => 1.0,
        Outcome::Win(_) => -1.0,
        Outcome::Draw => 0.0,
    }
}

fn rollout_value(obs: &Observation, max_depth: u32, rng: &mut dyn rand::RngCore) -> f32 {
    let mut state = obs.sample_state(rng);
    let mut depth = 0;
    while state.outcome().is_none() && depth < max_depth {
        let legal = state.legal_actions();
        let a = *legal.choose(rng).expect("non-terminal state has actions");
        state.apply(&a, rng);
        depth += 1;
    }
    match state.outcome() {
        Some(o) => outcome_value(o, obs.to_move),
        None => {
            // Heuristic for truncated rollouts: normalised life difference.
            let me = state.entity(obs.to_move).lifepool as f32;
            let them = state.entity(obs.to_move.opponent()).lifepool as f32;
            ((me - them) / 20.0).clamp(-1.0, 1.0)
        }
    }
}

