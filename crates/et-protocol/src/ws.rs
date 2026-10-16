//! JSON messages exchanged over the WebSocket between web client and server.
//! Tagged enums keep the wire self-describing and easy to inspect.

use et_engine::{Action, Archetype, CardKind, CardType, Observation, Outcome, Player};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Start a human-vs-AI game. `seat` is the human's seat.
    NewGame { difficulty: String, seat: Player },
    /// Start an AI-vs-AI game to spectate.
    Spectate { first: String, second: String },
    Act { action: Action },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardInfo {
    pub kind: CardKind,
    pub name: String,
    pub card_type: CardType,
    pub archetype: Archetype,
    pub cost: u32,
    pub text: String,
}

impl CardInfo {
    pub fn all() -> Vec<CardInfo> {
        CardKind::ALL
            .iter()
            .map(|k| {
                let d = k.def();
                CardInfo { kind: *k, name: d.name.into(), card_type: d.card_type, archetype: d.archetype, cost: d.cost, text: d.text.into() }
            })
            .collect()
    }
}

/// One option the AI weighed. Mirrors `et_search::Candidate` without
/// making the protocol crate depend on the search crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateInfo {
    pub action: Action,
    pub prior: f32,
    pub visits: u32,
    pub value: f32,
}

/// One public attribute that an action changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatChange {
    pub player: Player,
    /// One of: life, block, vulnerable, resource, stand_ground, fury.
    pub stat: String,
    pub before: i32,
    pub after: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Sent once on connect: static data the client needs to render.
    Catalog { cards: Vec<CardInfo>, difficulties: Vec<String> },
    GameStarted { game_id: String, you: Option<Player>, first: String, second: String },
    /// Observation from `observation.viewer`'s perspective. `legal` is
    /// empty unless the receiving client may act.
    State { observation: Observation, legal: Vec<Action> },
    /// Everything the AI considered before acting.
    AiDecision {
        player: Player,
        difficulty: String,
        iterations: u32,
        elapsed_ms: u64,
        candidates: Vec<CandidateInfo>,
        chosen: Action,
    },
    /// An action was applied. `changes` lists the public attributes it
    /// altered; `summary` is the same information as readable lines.
    Acted { player: Player, action: Action, changes: Vec<StatChange>, summary: Vec<String> },
    GameOver { outcome: Outcome },
    Error { message: String },
}
