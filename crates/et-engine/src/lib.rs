//! Pure game rules for Emergent Tactics. See `docs/rules.md` for the
//! authoritative rules text this crate implements.
//!
//! This crate owns the game state, legal actions, transitions, each player's
//! observation (information set), determinization of an observation into a
//! concrete state, and the tensor encoding used by the network. It performs
//! no I/O; all randomness comes through an injected RNG.

pub mod cards;
pub mod encode;
pub mod observation;
pub mod state;

pub use cards::{Archetype, CardKind, CardType};
pub use observation::{Observation, PublicEntity};
pub use state::{Action, Entity, GameState, Outcome, Player, MAX_ROUNDS};
