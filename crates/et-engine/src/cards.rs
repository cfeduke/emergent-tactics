use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardType {
    Attack,
    Skill,
    Power,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Archetype {
    All,
    Warrior,
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
    pub archetype: Archetype,
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
    CardDef { kind: CardKind::Strike, name: "Strike", card_type: CardType::Attack, archetype: Archetype::All, cost: 1, text: "Deal 4 damage." },
    CardDef { kind: CardKind::Defend, name: "Defend", card_type: CardType::Skill, archetype: Archetype::All, cost: 1, text: "Gain 3 Block." },
    CardDef { kind: CardKind::StandGround, name: "Stand Ground", card_type: CardType::Skill, archetype: Archetype::Warrior, cost: 2, text: "At the start of your next turn gain +2 resource limit and +2 Block." },
    CardDef { kind: CardKind::Charge, name: "Charge!", card_type: CardType::Attack, archetype: Archetype::Warrior, cost: 2, text: "Deal 4 damage, then apply Vulnerable 2." },
    CardDef { kind: CardKind::Fury, name: "Fury", card_type: CardType::Power, archetype: Archetype::Warrior, cost: 1, text: "Exile. At the start of each of your turns lose 2 life; if still above 0, gain +1 resource limit this turn." },
];

pub struct ArchetypeDef {
    pub archetype: Archetype,
    pub lifepool: i32,
    pub draw_amount: u32,
    pub resource_limit: u32,
    pub deck: &'static [CardKind],
}

pub static WARRIOR: ArchetypeDef = ArchetypeDef {
    archetype: Archetype::Warrior,
    lifepool: 20,
    draw_amount: 4,
    resource_limit: 3,
    deck: &[
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
};

impl Archetype {
    pub fn def(self) -> &'static ArchetypeDef {
        match self {
            Archetype::Warrior | Archetype::All => &WARRIOR,
        }
    }
}
