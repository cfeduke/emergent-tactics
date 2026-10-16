//! Python bindings. Self-play in `training/` drives the Rust engine and
//! encoder through this module so the rules and feature layout are never
//! duplicated in Python.

use et_engine::{GameState, Outcome, Player, encode};
use pyo3::prelude::*;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[pyclass(name = "Game")]
struct PyGame {
    state: GameState,
    rng: ChaCha8Rng,
}

#[pymethods]
impl PyGame {
    #[new]
    #[pyo3(signature = (seed = 0))]
    fn new(seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        Self { state: GameState::new(&mut rng), rng }
    }

    fn turn(&self) -> u32 {
        self.state.turn
    }

    fn round(&self) -> u32 {
        self.state.round
    }

    /// 0 for the first player, 1 for the second.
    fn to_move(&self) -> u8 {
        self.state.to_move.index() as u8
    }

    fn legal_actions(&self) -> Vec<usize> {
        self.state.legal_actions().iter().map(encode::action_index).collect()
    }

    fn apply(&mut self, action_index: usize) -> PyResult<()> {
        let action = encode::action_from_index(action_index)
            .filter(|a| self.state.is_legal(a))
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("illegal action"))?;
        self.state.apply(&action, &mut self.rng);
        Ok(())
    }

    fn is_over(&self) -> bool {
        self.state.outcome().is_some()
    }

    /// +1 if the first player won, -1 if the second, 0 for a draw; None if ongoing.
    fn outcome(&self) -> Option<f32> {
        self.state.outcome().map(|o| match o {
            Outcome::Win(Player::First) => 1.0,
            Outcome::Win(Player::Second) => -1.0,
            Outcome::Draw => 0.0,
        })
    }

    /// Feature vector for the player to move.
    fn encode(&self) -> Vec<f32> {
        encode::encode_observation(&self.state.observe(self.state.to_move))
    }

    fn observation_json(&self) -> String {
        serde_json::to_string(&self.state.observe(self.state.to_move)).expect("serializable")
    }
}

#[pymodule]
fn emergent_tactics(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGame>()?;
    m.add("FEATURE_LEN", encode::FEATURE_LEN)?;
    m.add("ACTION_SPACE", encode::ACTION_SPACE)?;
    m.add("ACTION_NAMES", action_names())?;
    Ok(())
}

fn action_names() -> Vec<String> {
    (0..encode::ACTION_SPACE)
        .map(|i| match encode::action_from_index(i) {
            Some(et_engine::Action::Play(k)) => k.def().name.to_string(),
            _ => "End Turn".to_string(),
        })
        .collect()
}
