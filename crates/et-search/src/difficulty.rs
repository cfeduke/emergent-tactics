use serde::{Deserialize, Serialize};

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
}

impl Difficulty {
    pub fn presets() -> Vec<Difficulty> {
        vec![
            Difficulty { name: "novice".into(), iterations: 50, determinizations: 5, temperature: 1.0, exploration: 1.4 },
            Difficulty { name: "adept".into(), iterations: 400, determinizations: 20, temperature: 0.5, exploration: 1.4 },
            Difficulty { name: "master".into(), iterations: 2000, determinizations: 50, temperature: 0.0, exploration: 1.4 },
        ]
    }

    pub fn by_name(name: &str) -> Option<Difficulty> {
        Self::presets().into_iter().find(|d| d.name == name)
    }
}
