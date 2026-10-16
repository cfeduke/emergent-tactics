use serde::{Deserialize, Serialize};

/// Which leaf evaluator a preset wants. The caller supplies the actual
/// evaluator; this records the intent and lets the server fall back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EvaluatorKind {
    #[default]
    Rollout,
    Network,
}

/// Knobs that together define an opponent's strength. Search budget and
/// temperature are varied at runtime; the evaluator (rollouts vs. which
/// network checkpoint) is chosen by the caller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Difficulty {
    pub name: String,
    pub iterations: u32,
    pub determinizations: u32,
    /// Sampling temperature over final visit counts. 0 = argmax.
    pub temperature: f32,
    pub exploration: f32,
    /// Dirichlet noise mixed into the root priors: (alpha, weight). Used by
    /// self-play for exploration; presets leave it off.
    #[serde(default)]
    pub root_noise: Option<(f32, f32)>,
    #[serde(default)]
    pub evaluator: EvaluatorKind,
}

impl Difficulty {
    /// Three budgets, each available with rollout or network evaluation so
    /// the two can be compared at equal search cost.
    pub fn presets() -> Vec<Difficulty> {
        let tiers = [("novice", 50, 5, 1.0), ("adept", 400, 20, 0.5), ("master", 2000, 50, 0.0)];
        let mut out = Vec::new();
        for (name, iterations, determinizations, temperature) in tiers {
            for (prefix, evaluator) in [("", EvaluatorKind::Rollout), ("net-", EvaluatorKind::Network)] {
                out.push(Difficulty {
                    name: format!("{prefix}{name}"),
                    iterations,
                    determinizations,
                    temperature,
                    exploration: 1.4,
                    root_noise: None,
                    evaluator,
                });
            }
        }
        out
    }

    pub fn by_name(name: &str) -> Option<Difficulty> {
        Self::presets().into_iter().find(|d| d.name == name)
    }
}
