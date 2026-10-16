//! JSON messages exchanged over the WebSocket between web client and server.
//! Tagged enums keep the wire self-describing and easy to inspect.

use et_engine::{Action, Archetype, Availability, CardKind, CardType, Observation, Outcome, Player, Polarity, StatusKind};
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
    pub archetype: Option<Archetype>,
    pub availability: Availability,
    pub cost: u32,
    pub text: String,
}

impl CardInfo {
    pub fn all() -> Vec<CardInfo> {
        CardKind::ALL
            .iter()
            .map(|k| {
                let d = k.def();
                CardInfo {
                    kind: *k,
                    name: d.name.into(),
                    card_type: d.card_type,
                    archetype: d.archetype,
                    availability: d.availability,
                    cost: d.cost,
                    text: d.text.into(),
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusInfo {
    pub kind: StatusKind,
    pub name: String,
    pub polarity: Polarity,
}

impl StatusInfo {
    pub fn all() -> Vec<StatusInfo> {
        StatusKind::ALL.iter().map(|k| StatusInfo { kind: *k, name: k.def().name.into(), polarity: k.def().polarity }).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchetypeInfo {
    pub archetype: Archetype,
    pub name: String,
    pub availability: Availability,
    pub lifepool: i32,
    pub draw_amount: u32,
    pub resource_limit: u32,
    pub pool: Vec<CardKind>,
}

impl ArchetypeInfo {
    pub fn all() -> Vec<ArchetypeInfo> {
        Archetype::ALL
            .iter()
            .map(|a| {
                let d = a.def();
                ArchetypeInfo {
                    archetype: *a,
                    name: d.name.into(),
                    availability: d.availability,
                    lifepool: d.lifepool,
                    draw_amount: d.draw_amount,
                    resource_limit: d.resource_limit,
                    pool: d.pool.to_vec(),
                }
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
    /// One of: life, block, resource, or `status:<StatusKind>`.
    pub stat: String,
    pub before: i32,
    pub after: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Sent once on connect: static data the client needs to render.
    Catalog {
        cards: Vec<CardInfo>,
        statuses: Vec<StatusInfo>,
        archetypes: Vec<ArchetypeInfo>,
        difficulties: Vec<String>,
        library_fingerprint: String,
    },
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
