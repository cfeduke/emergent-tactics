//! Information Set Monte Carlo Tree Search (ISMCTS).
//!
//! The search operates on an [`Observation`] (what the searching player can
//! see), sampling a concrete [`GameState`] consistent with it on each
//! iteration (a determinization). Leaf evaluation is delegated to an
//! [`Evaluator`], which is either random rollouts or a neural network.

pub mod difficulty;
pub mod evaluator;
pub mod ismcts;

pub use difficulty::Difficulty;
pub use evaluator::{Evaluation, Evaluator, RolloutEvaluator};
pub use ismcts::{Candidate, SearchResult, SearchStats, search};
