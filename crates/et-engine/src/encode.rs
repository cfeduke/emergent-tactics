//! Tensor encoding of an [`Observation`] for the policy/value network.
//!
//! This is the single source of truth for the feature layout. The Python
//! training code calls this through the `et-py` bindings rather than
//! reimplementing it, so Rust inference and PyTorch training can never
//! disagree about what each input index means.

use crate::{Action, Observation};

/// Length of the flat feature vector produced by [`encode_observation`].
pub const FEATURE_LEN: usize = 2;

/// Size of the fixed action space the policy head predicts over.
pub const ACTION_SPACE: usize = 1;

pub fn encode_observation(obs: &Observation) -> Vec<f32> {
    vec![
        if obs.to_move == obs.viewer { 1.0 } else { 0.0 },
        obs.turn as f32,
    ]
}

/// Index of an action within the policy head's output vector.
pub fn action_index(action: &Action) -> usize {
    match action {
        Action::Pass => 0,
    }
}
