//! Pure game rules for Emergent Tactics.
//!
//! This crate owns the game state, the legal-action set, state transitions,
//! and the per-player observation (information set). It performs no I/O and
//! takes all randomness through an injected RNG so that search and training
//! are reproducible.
//!
//! Everything downstream (search, server, Python bindings) depends on this
//! crate; it depends on nothing else in the workspace.

pub mod encode;
pub mod state;

pub use state::{Action, GameState, Observation, Outcome, Player};
