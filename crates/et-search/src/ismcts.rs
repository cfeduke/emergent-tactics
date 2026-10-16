use et_engine::{Action, Observation};
use rand::seq::IndexedRandom;

use crate::{Difficulty, Evaluator};

#[derive(Debug, Clone)]
pub struct SearchStats {
    pub iterations: u32,
    pub visits: Vec<(Action, u32)>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub action: Action,
    pub stats: SearchStats,
}

/// Run ISMCTS from `obs` and return the chosen action with visit statistics.
///
/// Placeholder: samples an action from the evaluator's priors. The real
/// tree search replaces this.
pub fn search(obs: &Observation, legal: &[Action], difficulty: &Difficulty, evaluator: &dyn Evaluator, rng: &mut dyn rand::RngCore) -> SearchResult {
    let eval = evaluator.evaluate(obs, legal, rng);
    let action = eval
        .priors
        .choose_weighted(rng, |(_, p)| *p)
        .map(|(a, _)| *a)
        .unwrap_or(Action::EndTurn);
    SearchResult {
        action,
        stats: SearchStats {
            iterations: difficulty.iterations,
            visits: legal.iter().map(|a| (*a, 0)).collect(),
        },
    }
}
