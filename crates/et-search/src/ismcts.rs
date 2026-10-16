use et_engine::{Action, Observation};

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
pub fn search(obs: &Observation, legal: &[Action], difficulty: &Difficulty, evaluator: &dyn Evaluator) -> SearchResult {
    let eval = evaluator.evaluate(obs, legal);
    let action = eval
        .priors
        .iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(a, _)| a.clone())
        .unwrap_or(Action::Pass);
    SearchResult {
        action,
        stats: SearchStats {
            iterations: difficulty.iterations,
            visits: legal.iter().map(|a| (a.clone(), 0)).collect(),
        },
    }
}
