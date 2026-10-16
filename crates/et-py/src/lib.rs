//! Python bindings. Self-play in `training/` drives the Rust engine and
//! encoder through this module so the rules and feature layout are never
//! duplicated in Python.

use et_engine::{Action, GameState, Player, encode};
use pyo3::prelude::*;

#[pyclass(name = "Game")]
struct PyGame {
    state: GameState,
}

#[pymethods]
impl PyGame {
    #[new]
    fn new() -> Self {
        Self { state: GameState::new() }
    }

    fn turn(&self) -> u32 {
        self.state.turn
    }

    fn to_move(&self) -> u8 {
        match self.state.to_move {
            Player::First => 0,
            Player::Second => 1,
        }
    }

    fn legal_actions(&self) -> Vec<usize> {
        self.state.legal_actions().iter().map(encode::action_index).collect()
    }

    fn apply(&mut self, action_index: usize) -> PyResult<()> {
        let action = self
            .state
            .legal_actions()
            .into_iter()
            .find(|a| encode::action_index(a) == action_index)
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("illegal action"))?;
        self.state.apply(&action);
        Ok(())
    }

    fn is_over(&self) -> bool {
        self.state.outcome().is_some()
    }

    /// Feature vector for the player to move.
    fn encode(&self) -> Vec<f32> {
        encode::encode_observation(&self.state.observe(self.state.to_move))
    }
}

#[pymodule]
fn emergent_tactics(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGame>()?;
    m.add("FEATURE_LEN", encode::FEATURE_LEN)?;
    m.add("ACTION_SPACE", encode::ACTION_SPACE)?;
    let _ = Action::Pass;
    Ok(())
}
