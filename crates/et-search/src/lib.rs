//! Information Set Monte Carlo Tree Search (ISMCTS).
//!
//! The search operates on an [`Observation`] (what the searching player can
//! see), sampling a concrete [`GameState`] consistent with it on each
//! iteration (a determinization). Leaf evaluation is delegated to an
//! [`Evaluator`], which is either random rollouts or a neural network.

pub mod arena;
pub mod difficulty;
pub mod evaluator;
pub mod ismcts;
pub mod network;
pub mod selfplay;

pub use arena::{ArenaStats, Controller, arena};
pub use difficulty::{Difficulty, EvaluatorKind};
pub use evaluator::{Evaluation, Evaluator, RolloutEvaluator};
pub use ismcts::{Candidate, SearchResult, SearchStats, search};
pub use network::{NetworkEvaluator, priors_from_logits};
pub use selfplay::{Sample, SelfPlayConfig, SelfPlayStats, self_play};
