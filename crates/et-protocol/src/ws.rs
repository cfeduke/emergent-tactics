//! JSON messages exchanged over the WebSocket between web client and server.
//! Tagged enums keep the wire self-describing and easy to inspect.

use et_engine::{Action, Observation, Outcome, Player};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Start a human-vs-AI game at the named difficulty.
    NewGame { difficulty: String },
    /// Start an AI-vs-AI game to spectate.
    Spectate { first: String, second: String },
    Act { action: Action },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    GameStarted { game_id: String, you: Option<Player> },
    State { observation: Observation, legal: Vec<Action> },
    /// Search progress while the AI thinks; useful for the debug panel.
    Thinking { player: Player, iterations: u32, visits: Vec<(Action, u32)> },
    Acted { player: Player, action: Action },
    GameOver { outcome: Outcome },
    Error { message: String },
}
