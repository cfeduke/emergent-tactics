//! Static definitions: cards, statuses, archetypes, and a fingerprint of the
//! whole library that trained models are pinned to.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardType {
    Attack,
    Skill,
    Power,
}

/// Who may put a card or archetype into a deck. Enforced at deck
/// construction only; the engine and model never consult it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Availability {
    Both,
    NpcOnly,
    PlayerOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Archetype {
    Warrior,
}

impl Archetype {
    pub const ALL: [Archetype; 1] = [Archetype::Warrior];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn def(self) -> &'static ArchetypeDef {
        &ARCHETYPES[self.index()]
    }
}

/// Every distinct card in the game. Duplicates in a deck are the same kind,
/// so this doubles as the policy head's action index (see `encode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CardKind {
    Strike,
    Defend,
    StandGround,
    Charge,
    Fury,
}

impl CardKind {
    pub const ALL: [CardKind; 5] = [
        CardKind::Strike,
        CardKind::Defend,
        CardKind::StandGround,
        CardKind::Charge,
        CardKind::Fury,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(i: usize) -> Option<CardKind> {
        Self::ALL.get(i).copied()
    }

    pub fn def(self) -> &'static CardDef {
        &CARDS[self.index()]
    }
}

pub struct CardDef {
    pub kind: CardKind,
    pub name: &'static str,
    pub card_type: CardType,
    /// `None` means any archetype may run it.
    pub archetype: Option<Archetype>,
    pub availability: Availability,
    pub cost: u32,
    pub text: &'static str,
}

impl CardDef {
    /// Powers always exile; other cards may in the future via a keyword.
    pub fn exiles(&self) -> bool {
        self.card_type == CardType::Power
    }
}

static CARDS: [CardDef; 5] = [
    CardDef { kind: CardKind::Strike, name: "Strike", card_type: CardType::Attack, archetype: None, availability: Availability::Both, cost: 1, text: "Deal 4 damage." },
    CardDef { kind: CardKind::Defend, name: "Defend", card_type: CardType::Skill, archetype: None, availability: Availability::Both, cost: 1, text: "Gain 3 Block." },
    CardDef { kind: CardKind::StandGround, name: "Stand Ground", card_type: CardType::Skill, archetype: Some(Archetype::Warrior), availability: Availability::Both, cost: 2, text: "At the start of your next turn gain +2 resource limit and +2 Block." },
    CardDef { kind: CardKind::Charge, name: "Charge!", card_type: CardType::Attack, archetype: Some(Archetype::Warrior), availability: Availability::Both, cost: 2, text: "Deal 4 damage, then apply Vulnerable 2." },
    CardDef { kind: CardKind::Fury, name: "Fury", card_type: CardType::Power, archetype: Some(Archetype::Warrior), availability: Availability::Both, cost: 1, text: "Exile. At the start of each of your turns lose 2 life; if still above 0, gain +1 resource limit this turn." },
];

/// Every buff and debuff. Declaration order is the order start-of-turn
/// effects fire in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum StatusKind {
    Fury,
    StandGround,
    Vulnerable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Polarity {
    Buff,
    Debuff,
}

/// How a status counter N evolves on its own. Card effects may also add to
/// or remove it directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tick {
    /// Lasts for the match.
    Never,
    /// N decreases by 1 at the end of every round, including the current one.
    EndOfRound,
    /// Fires once at the owner's next turn start, then is removed.
    ConsumedAtTurnStart,
}

pub struct StatusDef {
    pub kind: StatusKind,
    pub name: &'static str,
    pub polarity: Polarity,
    pub tick: Tick,
}

impl StatusKind {
    pub const ALL: [StatusKind; 3] = [StatusKind::Fury, StatusKind::StandGround, StatusKind::Vulnerable];
    pub const COUNT: usize = Self::ALL.len();

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn def(self) -> &'static StatusDef {
        &STATUSES[self.index()]
    }
}

static STATUSES: [StatusDef; 3] = [
    StatusDef { kind: StatusKind::Fury, name: "Fury", polarity: Polarity::Buff, tick: Tick::Never },
    StatusDef { kind: StatusKind::StandGround, name: "Stand Ground", polarity: Polarity::Buff, tick: Tick::ConsumedAtTurnStart },
    StatusDef { kind: StatusKind::Vulnerable, name: "Vulnerable", polarity: Polarity::Debuff, tick: Tick::EndOfRound },
];

pub struct ArchetypeDef {
    pub archetype: Archetype,
    pub name: &'static str,
    pub availability: Availability,
    pub lifepool: i32,
    pub draw_amount: u32,
    pub resource_limit: u32,
    /// Statuses the entity starts the match with.
    pub starting_statuses: &'static [(StatusKind, u32)],
    /// Cards a deck of this archetype may contain. The determinizer samples
    /// unseen opponent cards from here.
    pub pool: &'static [CardKind],
    /// The stock deck used when none is supplied.
    pub default_deck: &'static [CardKind],
}

static ARCHETYPES: [ArchetypeDef; 1] = [ArchetypeDef {
    archetype: Archetype::Warrior,
    name: "Warrior",
    availability: Availability::Both,
    lifepool: 20,
    draw_amount: 4,
    resource_limit: 3,
    starting_statuses: &[],
    pool: &[CardKind::Strike, CardKind::Defend, CardKind::StandGround, CardKind::Charge, CardKind::Fury],
    default_deck: &[
        CardKind::Strike,
        CardKind::Strike,
        CardKind::Strike,
        CardKind::Strike,
        CardKind::Defend,
        CardKind::Defend,
        CardKind::Defend,
        CardKind::StandGround,
        CardKind::Charge,
        CardKind::Fury,
    ],
}];

/// Stable 64-bit FNV-1a digest of everything a trained model depends on:
/// card, status and archetype definitions plus the encoder dimensions.
/// Exported with each model and checked by the server before serving.
pub fn library_fingerprint() -> u64 {
    let mut s = String::new();
    for c in &CARDS {
        s.push_str(&format!("card|{:?}|{}|{:?}|{:?}|{:?}|{}|{}\n", c.kind, c.name, c.card_type, c.archetype, c.availability, c.cost, c.text));
    }
    for st in &STATUSES {
        s.push_str(&format!("status|{:?}|{}|{:?}|{:?}\n", st.kind, st.name, st.polarity, st.tick));
    }
    for a in &ARCHETYPES {
        s.push_str(&format!(
            "arch|{:?}|{}|{:?}|{}|{}|{}|{:?}|{:?}|{:?}\n",
            a.archetype, a.name, a.availability, a.lifepool, a.draw_amount, a.resource_limit, a.starting_statuses, a.pool, a.default_deck
        ));
    }
    s.push_str(&format!("encode|{}|{}\n", crate::encode::FEATURE_LEN, crate::encode::ACTION_SPACE));
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn library_fingerprint_hex() -> String {
    format!("{:016x}", library_fingerprint())
}
