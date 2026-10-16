//! What one player can see, and how to sample a full state consistent with it.

use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

use crate::cards::{Archetype, CardKind};
use crate::state::{Entity, GameState, Player};

/// Public attributes of either entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicEntity {
    pub archetype: Archetype,
    pub lifepool: i32,
    pub block: u32,
    pub resource: u32,
    pub resource_limit: u32,
    pub resource_bonus: u32,
    pub vulnerable: u32,
    pub fury: bool,
    pub stand_ground_pending: u32,
    pub hand_count: usize,
    pub draw_count: usize,
    pub discard_face_up: Vec<CardKind>,
    pub discard_face_down_count: usize,
    pub exile: Vec<CardKind>,
    pub played_this_turn: Vec<CardKind>,
}

impl PublicEntity {
    fn of(e: &Entity) -> Self {
        Self {
            archetype: e.archetype,
            lifepool: e.lifepool,
            block: e.block,
            resource: e.resource,
            resource_limit: e.resource_limit,
            resource_bonus: e.resource_bonus,
            vulnerable: e.vulnerable,
            fury: e.fury,
            stand_ground_pending: e.stand_ground_pending,
            hand_count: e.hand.len(),
            draw_count: e.draw_pile.len(),
            discard_face_up: e.discard_face_up.clone(),
            discard_face_down_count: e.discard_face_down.len(),
            exile: e.exile.clone(),
            played_this_turn: e.played_this_turn.clone(),
        }
    }
}

/// The information set for `viewer`. Two states that produce equal
/// observations are indistinguishable to that player.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Observation {
    pub viewer: Player,
    pub to_move: Player,
    pub round: u32,
    pub turn: u32,
    pub me: PublicEntity,
    pub opponent: PublicEntity,
    /// Viewer's hand, sorted so equal hands compare equal.
    pub hand: Vec<CardKind>,
    /// Viewer's own face-down discards, sorted. The owner remembers them.
    pub discard_face_down: Vec<CardKind>,
}

impl GameState {
    pub fn observe(&self, viewer: Player) -> Observation {
        let me = self.entity(viewer);
        let mut hand = me.hand.clone();
        hand.sort();
        let mut discard_face_down = me.discard_face_down.clone();
        discard_face_down.sort();
        Observation {
            viewer,
            to_move: self.to_move,
            round: self.round,
            turn: self.turn,
            me: PublicEntity::of(me),
            opponent: PublicEntity::of(self.entity(viewer.opponent())),
            hand,
            discard_face_down,
        }
    }
}

/// Multiset subtraction: `pool` minus `taken`, in sorted order.
fn remaining(pool: &[CardKind], taken: impl Iterator<Item = CardKind>) -> Vec<CardKind> {
    let mut left = pool.to_vec();
    for t in taken {
        if let Some(i) = left.iter().position(|c| *c == t) {
            left.swap_remove(i);
        }
    }
    left.sort();
    left
}

impl Observation {
    /// Sample a full [`GameState`] consistent with this observation
    /// (a determinization). The viewer's draw pile is shuffled; the
    /// opponent's hand, face-down discard and draw pile are filled from the
    /// cards not otherwise accounted for, then shuffled.
    pub fn sample_state(&self, rng: &mut (impl Rng + ?Sized)) -> GameState {
        let me = self.sample_me(rng);
        let opp = self.sample_opponent(rng);
        let entities = match self.viewer {
            Player::First => [me, opp],
            Player::Second => [opp, me],
        };
        GameState::from_parts(entities, self.to_move, self.round, self.turn)
    }

    fn sample_me(&self, rng: &mut (impl Rng + ?Sized)) -> Entity {
        let deck = self.me.archetype.def().deck;
        let known = self
            .hand
            .iter()
            .chain(self.discard_face_down.iter())
            .chain(self.me.discard_face_up.iter())
            .chain(self.me.exile.iter())
            .copied();
        let mut draw_pile = remaining(deck, known);
        debug_assert_eq!(draw_pile.len(), self.me.draw_count);
        draw_pile.shuffle(rng);
        entity_from(&self.me, draw_pile, self.hand.clone(), self.discard_face_down.clone())
    }

    fn sample_opponent(&self, rng: &mut (impl Rng + ?Sized)) -> Entity {
        let o = &self.opponent;
        let deck = o.archetype.def().deck;
        let known = o.discard_face_up.iter().chain(o.exile.iter()).copied();
        let mut unknown = remaining(deck, known);
        debug_assert_eq!(unknown.len(), o.hand_count + o.discard_face_down_count + o.draw_count);
        unknown.shuffle(rng);
        let hand = unknown.split_off(unknown.len() - o.hand_count);
        let face_down = unknown.split_off(unknown.len() - o.discard_face_down_count);
        entity_from(o, unknown, hand, face_down)
    }
}

fn entity_from(p: &PublicEntity, draw_pile: Vec<CardKind>, hand: Vec<CardKind>, discard_face_down: Vec<CardKind>) -> Entity {
    Entity {
        archetype: p.archetype,
        lifepool: p.lifepool,
        block: p.block,
        resource: p.resource,
        resource_limit: p.resource_limit,
        resource_bonus: p.resource_bonus,
        draw_amount: p.archetype.def().draw_amount,
        vulnerable: p.vulnerable,
        fury: p.fury,
        stand_ground_pending: p.stand_ground_pending,
        draw_pile,
        hand,
        discard_face_up: p.discard_face_up.clone(),
        discard_face_down,
        exile: p.exile.clone(),
        played_this_turn: p.played_this_turn.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Action;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn determinization_matches_observation_and_hides_opponent_order() {
        let mut r = ChaCha8Rng::seed_from_u64(3);
        let mut s = GameState::new(&mut r);
        // Play a few turns so every zone is populated.
        for _ in 0..3 {
            let a = s.legal_actions()[0];
            s.apply(&a, &mut r);
        }
        s.apply(&Action::EndTurn, &mut r);
        let viewer = s.to_move;
        let obs = s.observe(viewer);
        for seed in 0..20 {
            let mut r2 = ChaCha8Rng::seed_from_u64(seed);
            let sampled = obs.sample_state(&mut r2);
            assert_eq!(sampled.observe(viewer), obs, "sample must be consistent");
            assert_eq!(sampled.to_move, s.to_move);
            // Opponent's zones are a permutation of the truth.
            let (t, u) = (s.entity(viewer.opponent()), sampled.entity(viewer.opponent()));
            let mut truth: Vec<_> = t.draw_pile.iter().chain(&t.hand).chain(&t.discard_face_down).copied().collect();
            let mut samp: Vec<_> = u.draw_pile.iter().chain(&u.hand).chain(&u.discard_face_down).copied().collect();
            truth.sort();
            samp.sort();
            assert_eq!(truth, samp);
        }
    }
}
