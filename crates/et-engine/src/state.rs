use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::cards::{Archetype, CardKind};

/// Full rounds after which the match is declared a draw.
pub const MAX_ROUNDS: u32 = 30;
pub const HAND_LIMIT: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Player {
    First,
    Second,
}

impl Player {
    pub fn opponent(self) -> Player {
        match self {
            Player::First => Player::Second,
            Player::Second => Player::First,
        }
    }
    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Win(Player),
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    Play(CardKind),
    EndTurn,
}

/// One participant's complete state, including zones hidden from the opponent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    pub archetype: Archetype,
    pub lifepool: i32,
    pub block: u32,
    pub resource: u32,
    pub resource_limit: u32,
    /// Additive bonuses to the limit that expire at end of turn.
    pub resource_bonus: u32,
    pub draw_amount: u32,
    pub vulnerable: u32,
    pub fury: bool,
    pub stand_ground_pending: u32,
    /// Top of the pile is the last element.
    pub draw_pile: Vec<CardKind>,
    pub hand: Vec<CardKind>,
    pub discard_face_up: Vec<CardKind>,
    pub discard_face_down: Vec<CardKind>,
    pub exile: Vec<CardKind>,
    pub played_this_turn: Vec<CardKind>,
}

impl Entity {
    pub fn new(archetype: Archetype, rng: &mut (impl Rng + ?Sized)) -> Self {
        let def = archetype.def();
        let mut draw_pile = def.deck.to_vec();
        draw_pile.shuffle(rng);
        Self {
            archetype,
            lifepool: def.lifepool,
            block: 0,
            resource: 0,
            resource_limit: def.resource_limit,
            resource_bonus: 0,
            draw_amount: def.draw_amount,
            vulnerable: 0,
            fury: false,
            stand_ground_pending: 0,
            draw_pile,
            hand: Vec::new(),
            discard_face_up: Vec::new(),
            discard_face_down: Vec::new(),
            exile: Vec::new(),
            played_this_turn: Vec::new(),
        }
    }

    pub fn effective_limit(&self) -> u32 {
        self.resource_limit + self.resource_bonus
    }

    fn draw_one(&mut self, rng: &mut (impl Rng + ?Sized)) {
        if self.draw_pile.is_empty() {
            self.draw_pile.append(&mut self.discard_face_up);
            self.draw_pile.append(&mut self.discard_face_down);
            self.draw_pile.shuffle(rng);
        }
        if let Some(card) = self.draw_pile.pop() {
            if self.hand.len() < HAND_LIMIT {
                self.hand.push(card);
            } else {
                self.discard_face_down.push(card);
            }
        }
    }

    /// Returns damage that reached lifepool.
    fn take_damage(&mut self, amount: u32) -> u32 {
        let amount = if self.vulnerable > 0 { amount + 1 } else { amount };
        let absorbed = amount.min(self.block);
        self.block -= absorbed;
        let through = amount - absorbed;
        self.lifepool -= through as i32;
        through
    }

    fn lose_life(&mut self, amount: u32) {
        self.lifepool -= amount as i32;
    }

    fn dead(&self) -> bool {
        self.lifepool <= 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub entities: [Entity; 2],
    pub to_move: Player,
    /// Completed full rounds.
    pub round: u32,
    /// Entity turns taken so far.
    pub turn: u32,
    outcome: Option<Outcome>,
}

impl GameState {
    pub fn new(rng: &mut (impl Rng + ?Sized)) -> Self {
        Self::with_archetypes(Archetype::Warrior, Archetype::Warrior, rng)
    }

    pub fn with_archetypes(first: Archetype, second: Archetype, rng: &mut (impl Rng + ?Sized)) -> Self {
        let mut s = Self {
            entities: [Entity::new(first, rng), Entity::new(second, rng)],
            to_move: Player::First,
            round: 0,
            turn: 0,
            outcome: None,
        };
        s.start_turn(rng);
        s
    }

    /// Build a state directly from already-prepared entities. Used by the
    /// determinizer; `start_turn` is not run.
    pub(crate) fn from_parts(entities: [Entity; 2], to_move: Player, round: u32, turn: u32) -> Self {
        let mut s = Self { entities, to_move, round, turn, outcome: None };
        s.check_deaths();
        s
    }

    pub fn entity(&self, p: Player) -> &Entity {
        &self.entities[p.index()]
    }

    pub fn current(&self) -> &Entity {
        self.entity(self.to_move)
    }

    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    pub fn legal_actions(&self) -> Vec<Action> {
        if self.outcome.is_some() {
            return Vec::new();
        }
        let me = self.current();
        let mut acts: Vec<Action> = Vec::with_capacity(CardKind::ALL.len() + 1);
        for kind in CardKind::ALL {
            if me.hand.contains(&kind) && kind.def().cost <= me.resource {
                acts.push(Action::Play(kind));
            }
        }
        acts.push(Action::EndTurn);
        acts
    }

    pub fn is_legal(&self, action: &Action) -> bool {
        self.legal_actions().contains(action)
    }

    /// Apply a legal action. Panics on an illegal one; callers validate first.
    pub fn apply(&mut self, action: &Action, rng: &mut (impl Rng + ?Sized)) {
        assert!(self.is_legal(action), "illegal action {action:?}");
        match action {
            Action::Play(kind) => self.play(*kind),
            Action::EndTurn => self.end_turn(rng),
        }
    }

    fn play(&mut self, kind: CardKind) {
        let (me_i, them_i) = (self.to_move.index(), self.to_move.opponent().index());
        let def = kind.def();
        {
            let me = &mut self.entities[me_i];
            let pos = me.hand.iter().position(|c| *c == kind).expect("card in hand");
            me.hand.remove(pos);
            me.resource -= def.cost;
            me.played_this_turn.push(kind);
        }
        match kind {
            CardKind::Strike => {
                self.entities[them_i].take_damage(4);
            }
            CardKind::Defend => {
                self.entities[me_i].block += 3;
            }
            CardKind::StandGround => {
                self.entities[me_i].stand_ground_pending += 1;
            }
            CardKind::Charge => {
                self.entities[them_i].take_damage(4);
                self.entities[them_i].vulnerable += 2;
            }
            CardKind::Fury => {
                self.entities[me_i].fury = true;
            }
        }
        let me = &mut self.entities[me_i];
        if def.exiles() {
            me.exile.push(kind);
        } else {
            me.discard_face_up.push(kind);
        }
        self.check_deaths();
    }

    fn end_turn(&mut self, rng: &mut (impl Rng + ?Sized)) {
        let me = &mut self.entities[self.to_move.index()];
        me.discard_face_down.append(&mut me.hand);
        me.resource_bonus = 0;
        me.resource = 0;
        me.played_this_turn.clear();
        if self.to_move == Player::Second {
            for e in &mut self.entities {
                e.vulnerable = e.vulnerable.saturating_sub(1);
            }
            self.round += 1;
            if self.round >= MAX_ROUNDS {
                self.outcome = Some(Outcome::Draw);
                return;
            }
        }
        self.turn += 1;
        self.to_move = self.to_move.opponent();
        self.start_turn(rng);
    }

    fn start_turn(&mut self, rng: &mut (impl Rng + ?Sized)) {
        let me = &mut self.entities[self.to_move.index()];
        me.block = 0;
        if me.fury {
            me.lose_life(2);
            if !me.dead() {
                me.resource_bonus += 1;
            }
        }
        if me.stand_ground_pending > 0 {
            me.resource_bonus += 2 * me.stand_ground_pending;
            me.block += 2 * me.stand_ground_pending;
            me.stand_ground_pending = 0;
        }
        me.resource = me.effective_limit();
        for _ in 0..me.draw_amount {
            me.draw_one(rng);
        }
        self.check_deaths();
    }

    fn check_deaths(&mut self) {
        if self.outcome.is_some() {
            return;
        }
        let dead = [self.entities[0].dead(), self.entities[1].dead()];
        self.outcome = match dead {
            [true, true] => Some(Outcome::Draw),
            [true, false] => Some(Outcome::Win(Player::Second)),
            [false, true] => Some(Outcome::Win(Player::First)),
            [false, false] => None,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn rng() -> ChaCha8Rng {
        ChaCha8Rng::seed_from_u64(7)
    }

    /// Put exact cards in the current player's hand and resource for scripted tests.
    fn rig(s: &mut GameState, p: Player, hand: &[CardKind], resource: u32) {
        let e = &mut s.entities[p.index()];
        e.hand = hand.to_vec();
        e.resource = resource;
    }

    #[test]
    fn first_turn_draws_four_and_refills_resource() {
        let s = GameState::new(&mut rng());
        assert_eq!(s.current().hand.len(), 4);
        assert_eq!(s.current().resource, 3);
        assert_eq!(s.entity(Player::Second).hand.len(), 0);
        assert_eq!(s.current().draw_pile.len(), 6);
    }

    #[test]
    fn strike_hits_block_then_life() {
        let mut r = rng();
        let mut s = GameState::new(&mut r);
        s.entities[1].block = 3;
        rig(&mut s, Player::First, &[CardKind::Strike], 3);
        s.apply(&Action::Play(CardKind::Strike), &mut r);
        assert_eq!(s.entity(Player::Second).block, 0);
        assert_eq!(s.entity(Player::Second).lifepool, 19);
        assert_eq!(s.current().resource, 2);
        assert_eq!(s.current().discard_face_up, vec![CardKind::Strike]);
    }

    #[test]
    fn charge_damage_resolves_before_vulnerable_and_ticks_per_round() {
        let mut r = rng();
        let mut s = GameState::new(&mut r);
        rig(&mut s, Player::First, &[CardKind::Charge, CardKind::Strike], 3);
        s.apply(&Action::Play(CardKind::Charge), &mut r);
        assert_eq!(s.entity(Player::Second).lifepool, 16, "charge itself not amplified");
        assert_eq!(s.entity(Player::Second).vulnerable, 2);
        s.apply(&Action::Play(CardKind::Strike), &mut r);
        assert_eq!(s.entity(Player::Second).lifepool, 11, "strike amplified by 1");
        s.apply(&Action::EndTurn, &mut r);
        // Second's turn in the same round: still Vulnerable 2.
        assert_eq!(s.entity(Player::Second).vulnerable, 2);
        s.apply(&Action::EndTurn, &mut r);
        // Round ended: 2 -> 1, lasts through First's next turn.
        assert_eq!(s.entity(Player::Second).vulnerable, 1);
        assert_eq!(s.round, 1);
        s.apply(&Action::EndTurn, &mut r);
        s.apply(&Action::EndTurn, &mut r);
        assert_eq!(s.entity(Player::Second).vulnerable, 0);
    }

    #[test]
    fn vulnerable_stacks_additively() {
        let mut r = rng();
        let mut s = GameState::new(&mut r);
        s.entities[1].vulnerable = 1;
        rig(&mut s, Player::First, &[CardKind::Charge], 3);
        s.apply(&Action::Play(CardKind::Charge), &mut r);
        assert_eq!(s.entity(Player::Second).vulnerable, 3);
    }

    #[test]
    fn block_resets_at_own_turn_start_but_protects_during_opponent_turn() {
        let mut r = rng();
        let mut s = GameState::new(&mut r);
        rig(&mut s, Player::First, &[CardKind::Defend], 3);
        s.apply(&Action::Play(CardKind::Defend), &mut r);
        assert_eq!(s.entity(Player::First).block, 3);
        s.apply(&Action::EndTurn, &mut r);
        assert_eq!(s.entity(Player::First).block, 3, "block persists into opponent turn");
        s.apply(&Action::EndTurn, &mut r);
        assert_eq!(s.entity(Player::First).block, 0, "block reset at own turn start");
    }

    #[test]
    fn stand_ground_fires_next_turn_after_block_reset_and_stacks() {
        let mut r = rng();
        let mut s = GameState::new(&mut r);
        rig(&mut s, Player::First, &[CardKind::StandGround, CardKind::StandGround], 4);
        s.apply(&Action::Play(CardKind::StandGround), &mut r);
        s.apply(&Action::Play(CardKind::StandGround), &mut r);
        s.apply(&Action::EndTurn, &mut r);
        s.apply(&Action::EndTurn, &mut r);
        let me = s.entity(Player::First);
        assert_eq!(me.block, 4);
        assert_eq!(me.resource, 7);
        assert_eq!(me.stand_ground_pending, 0);
        // Bonus is temporary.
        s.apply(&Action::EndTurn, &mut r);
        assert_eq!(s.entity(Player::First).resource_bonus, 0);
    }

    #[test]
    fn fury_is_exiled_costs_life_directly_and_gives_flat_bonus() {
        let mut r = rng();
        let mut s = GameState::new(&mut r);
        rig(&mut s, Player::First, &[CardKind::Fury], 3);
        s.apply(&Action::Play(CardKind::Fury), &mut r);
        assert_eq!(s.entity(Player::First).exile, vec![CardKind::Fury]);
        assert!(s.entity(Player::First).discard_face_up.is_empty());
        s.apply(&Action::EndTurn, &mut r);
        s.apply(&Action::EndTurn, &mut r);
        let me = s.entity(Player::First);
        assert_eq!(me.lifepool, 18);
        assert_eq!(me.resource, 4);
        s.apply(&Action::EndTurn, &mut r);
        s.apply(&Action::EndTurn, &mut r);
        let me = s.entity(Player::First);
        assert_eq!(me.lifepool, 16);
        assert_eq!(me.resource, 4, "flat, not cumulative");
    }

    #[test]
    fn fury_can_kill_its_owner_at_turn_start() {
        let mut r = rng();
        let mut s = GameState::new(&mut r);
        s.entities[0].lifepool = 2;
        rig(&mut s, Player::First, &[CardKind::Fury], 3);
        s.apply(&Action::Play(CardKind::Fury), &mut r);
        s.apply(&Action::EndTurn, &mut r);
        s.apply(&Action::EndTurn, &mut r);
        assert_eq!(s.outcome(), Some(Outcome::Win(Player::Second)));
        assert!(s.legal_actions().is_empty());
    }

    #[test]
    fn unplayed_cards_discard_face_down_and_reshuffle_when_empty() {
        let mut r = rng();
        let mut s = GameState::new(&mut r);
        // Turn 1: play nothing.
        s.apply(&Action::EndTurn, &mut r);
        assert_eq!(s.entity(Player::First).discard_face_down.len(), 4);
        s.apply(&Action::EndTurn, &mut r);
        // Turn 2 for First: 2 left in pile, so 2 drawn then reshuffle of 4+4 and 2 more.
        s.apply(&Action::EndTurn, &mut r);
        s.apply(&Action::EndTurn, &mut r);
        let me = s.entity(Player::First);
        assert_eq!(me.hand.len(), 4);
        assert_eq!(me.draw_pile.len() + me.hand.len() + me.discard_face_down.len() + me.discard_face_up.len(), 10);
    }

    #[test]
    fn legal_actions_respect_hand_and_resource() {
        let mut r = rng();
        let mut s = GameState::new(&mut r);
        rig(&mut s, Player::First, &[CardKind::Strike, CardKind::Strike, CardKind::Charge], 1);
        assert_eq!(s.legal_actions(), vec![Action::Play(CardKind::Strike), Action::EndTurn]);
    }

    #[test]
    fn round_cap_is_a_draw() {
        let mut r = rng();
        let mut s = GameState::new(&mut r);
        while s.outcome().is_none() {
            s.apply(&Action::EndTurn, &mut r);
        }
        assert_eq!(s.outcome(), Some(Outcome::Draw));
        assert_eq!(s.round, MAX_ROUNDS);
    }
}
