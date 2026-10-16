use std::time::Instant;

use et_engine::{Action, Observation};
use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};

use crate::{Difficulty, Evaluator};

/// Everything the search knew about one candidate action. Sent to the
/// client verbatim so the AI's reasoning can be inspected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub action: Action,
    pub prior: f32,
    pub visits: u32,
    /// Mean value from the perspective of the player to move.
    pub value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStats {
    pub iterations: u32,
    pub elapsed_ms: u64,
    pub candidates: Vec<Candidate>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub action: Action,
    pub stats: SearchStats,
}

/// Run ISMCTS from `obs` and return the chosen action with statistics.
///
/// Placeholder: samples an action from the evaluator's priors. The real
/// tree search replaces this.
pub fn search(obs: &Observation, legal: &[Action], difficulty: &Difficulty, evaluator: &dyn Evaluator, rng: &mut dyn rand::RngCore) -> SearchResult {
    let started = Instant::now();
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
            elapsed_ms: started.elapsed().as_millis() as u64,
            candidates: eval
                .priors
                .iter()
                .map(|(a, p)| Candidate { action: *a, prior: *p, visits: 0, value: eval.value })
                .collect(),
        },
    }
}
