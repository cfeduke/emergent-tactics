//! What one player can see, and how to sample a full state consistent with it.

use rand::Rng;
use rand::seq::{IndexedRandom, SliceRandom};
use serde::{Deserialize, Serialize};

use crate::cards::{Archetype, CardKind, StatusKind};
use crate::state::{CopyId, Entity, GameState, Player};

/// Public attributes of either entity. Deck contents are *not* public:
/// only cards that have been played are known, and after a reshuffle those
/// are known to exist somewhere in a hidden zone (`seen_hidden`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PublicEntity {
    pub archetype: Archetype,
    pub lifepool: i32,
    pub block: u32,
    pub resource: u32,
    pub resource_limit: u32,
    pub resource_bonus: u32,
    /// Non-zero statuses only, in `StatusKind` order.
    pub statuses: Vec<(StatusKind, u32)>,
    pub deck_size: usize,
    pub hand_count: usize,
    pub draw_count: usize,
    pub discard_face_up: Vec<CardKind>,
    pub discard_face_down_count: usize,
    pub exile: Vec<CardKind>,
    /// Cards previously revealed that are now in the hand, face-down discard
    /// or draw pile (zone unknown). Sorted.
    pub seen_hidden: Vec<CardKind>,
    pub played_this_turn: Vec<CardKind>,
}

impl PublicEntity {
    pub fn of(e: &Entity) -> Self {
        let mut seen_hidden: Vec<CardKind> = e
            .hand
            .iter()
            .chain(&e.discard_face_down)
            .chain(&e.draw_pile)
            .filter(|&&id| e.seen[id as usize])
            .map(|&id| e.kind(id))
            .collect();
        seen_hidden.sort();
        Self {
            archetype: e.archetype,
            lifepool: e.lifepool,
            block: e.block,
            resource: e.resource,
            resource_limit: e.resource_limit,
            resource_bonus: e.resource_bonus,
            statuses: StatusKind::ALL.iter().filter(|k| e.status(**k) > 0).map(|k| (*k, e.status(*k))).collect(),
            deck_size: e.deck_list.len(),
            hand_count: e.hand.len(),
            draw_count: e.draw_pile.len(),
            discard_face_up: e.kinds(&e.discard_face_up),
            discard_face_down_count: e.discard_face_down.len(),
            exile: e.kinds(&e.exile),
            seen_hidden,
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
    /// Viewer's own full deck list, sorted.
    pub deck: Vec<CardKind>,
    /// Viewer's hand, sorted so equal hands compare equal.
    pub hand: Vec<CardKind>,
    /// Viewer's own face-down discards, sorted. The owner remembers them.
    pub discard_face_down: Vec<CardKind>,
}

impl GameState {
    pub fn observe(&self, viewer: Player) -> Observation {
        let me = self.entity(viewer);
        let mut deck = me.deck_list.clone();
        deck.sort();
        let mut hand = me.kinds(&me.hand);
        hand.sort();
        let mut discard_face_down = me.kinds(&me.discard_face_down);
        discard_face_down.sort();
        Observation {
            viewer,
            to_move: self.to_move,
            round: self.round,
            turn: self.turn,
            me: PublicEntity::of(me),
            opponent: PublicEntity::of(self.entity(viewer.opponent())),
            deck,
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

/// Zones expressed as kinds, used to assemble a determinized entity.
struct Zones {
    hand: Vec<CardKind>,
    discard_face_down: Vec<CardKind>,
    draw_pile: Vec<CardKind>,
    /// Which of the hidden-zone cards (hand, face-down, draw, in that
    /// order) the opponent has previously seen.
    hidden_seen: Vec<bool>,
}

impl Observation {
    /// Sample a full [`GameState`] consistent with this observation
    /// (a determinization). The viewer's draw pile is shuffled. The
    /// opponent's hidden zones are filled with the cards known to be there
    /// plus cards sampled from the opponent archetype's pool, then shuffled.
    /// The opponent's true deck list is never consulted.
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
        let known = self
            .hand
            .iter()
            .chain(self.discard_face_down.iter())
            .chain(self.me.discard_face_up.iter())
            .chain(self.me.exile.iter())
            .copied();
        let mut draw_pile = remaining(&self.deck, known);
        debug_assert_eq!(draw_pile.len(), self.me.draw_count);
        draw_pile.shuffle(rng);
        // The opponent has seen some of my hidden cards; which copies does
        // not matter for play, only the multiset, so mark the first matches.
        let hidden: Vec<CardKind> = self.hand.iter().chain(&self.discard_face_down).chain(&draw_pile).copied().collect();
        let hidden_seen = mark_seen(&hidden, &self.me.seen_hidden);
        entity_from(&self.me, Zones { hand: self.hand.clone(), discard_face_down: self.discard_face_down.clone(), draw_pile, hidden_seen })
    }

    fn sample_opponent(&self, rng: &mut (impl Rng + ?Sized)) -> Entity {
        let o = &self.opponent;
        let hidden_n = o.hand_count + o.discard_face_down_count + o.draw_count;
        debug_assert!(o.seen_hidden.len() <= hidden_n);
        let pool = o.archetype.def().pool;
        let mut hidden: Vec<(CardKind, bool)> = o.seen_hidden.iter().map(|k| (*k, true)).collect();
        while hidden.len() < hidden_n {
            hidden.push((*pool.choose(rng).expect("non-empty pool"), false));
        }
        hidden.shuffle(rng);
        let mut rest = hidden;
        let draw: Vec<_> = rest.split_off(rest.len() - o.draw_count);
        let face_down: Vec<_> = rest.split_off(rest.len() - o.discard_face_down_count);
        let hand = rest;
        let hidden_seen = hand.iter().chain(&face_down).chain(&draw).map(|(_, s)| *s).collect();
        entity_from(
            o,
            Zones {
                hand: hand.into_iter().map(|(k, _)| k).collect(),
                discard_face_down: face_down.into_iter().map(|(k, _)| k).collect(),
                draw_pile: draw.into_iter().map(|(k, _)| k).collect(),
                hidden_seen,
            },
        )
    }
}

fn mark_seen(hidden: &[CardKind], seen: &[CardKind]) -> Vec<bool> {
    let mut want = seen.to_vec();
    hidden
        .iter()
        .map(|k| {
            if let Some(i) = want.iter().position(|w| w == k) {
                want.swap_remove(i);
                true
            } else {
                false
            }
        })
        .collect()
}

/// Assemble an entity whose `deck_list` is laid out as
/// face-up discard, exile, hand, face-down discard, draw pile.
fn entity_from(p: &PublicEntity, z: Zones) -> Entity {
    let mut deck_list: Vec<CardKind> = Vec::with_capacity(p.deck_size);
    let mut seen: Vec<bool> = Vec::with_capacity(p.deck_size);
    let mut ids = |kinds: &[CardKind], s: &[bool]| -> Vec<CopyId> {
        let start = deck_list.len();
        deck_list.extend_from_slice(kinds);
        seen.extend_from_slice(s);
        (start..deck_list.len()).map(|i| i as CopyId).collect()
    };
    let discard_face_up = ids(&p.discard_face_up, &vec![true; p.discard_face_up.len()]);
    let exile = ids(&p.exile, &vec![true; p.exile.len()]);
    let (hs, rest) = z.hidden_seen.split_at(z.hand.len());
    let (fs, ds) = rest.split_at(z.discard_face_down.len());
    let hand = ids(&z.hand, hs);
    let discard_face_down = ids(&z.discard_face_down, fs);
    let draw_pile = ids(&z.draw_pile, ds);
    debug_assert_eq!(deck_list.len(), p.deck_size);
    let mut statuses = [0; StatusKind::COUNT];
    for (k, n) in &p.statuses {
        statuses[k.index()] = *n;
    }
    Entity {
        archetype: p.archetype,
        deck_list,
        seen,
        lifepool: p.lifepool,
        block: p.block,
        resource: p.resource,
        resource_limit: p.resource_limit,
        resource_bonus: p.resource_bonus,
        draw_amount: p.archetype.def().draw_amount,
        statuses,
        draw_pile,
        hand,
        discard_face_up,
        discard_face_down,
        exile,
        played_this_turn: p.played_this_turn.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Action;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn play_some(s: &mut GameState, r: &mut ChaCha8Rng, turns: usize) {
        for _ in 0..turns {
            while s.outcome().is_none() {
                let legal = s.legal_actions();
                let a = legal[0];
                s.apply(&a, r);
                if a == Action::EndTurn {
                    break;
                }
            }
        }
    }

    #[test]
    fn determinization_is_consistent_and_never_reads_the_true_deck() {
        let mut r = ChaCha8Rng::seed_from_u64(3);
        // Give the second player a deck that differs from the pool's stock
        // deck so a sampler that peeked would be caught.
        let w = Archetype::Warrior;
        let odd = [CardKind::Charge, CardKind::Charge, CardKind::Charge, CardKind::Fury, CardKind::Strike, CardKind::Strike];
        let mut s = GameState::with_decks((w, w.def().default_deck), (w, &odd), &mut r);
        play_some(&mut s, &mut r, 5);
        let viewer = Player::First;
        let obs = s.observe(viewer);
        let mut saw_non_charge_in_unknown = false;
        for seed in 0..30 {
            let mut r2 = ChaCha8Rng::seed_from_u64(seed);
            let sampled = obs.sample_state(&mut r2);
            assert_eq!(sampled.observe(viewer), obs, "sample must be consistent");
            let opp = sampled.entity(Player::Second);
            assert_eq!(opp.deck_list.len(), odd.len());
            // Unseen hidden copies are sampled from the pool, so across
            // samples they should not always match the true deck.
            for (i, k) in opp.deck_list.iter().enumerate() {
                if !opp.seen[i] && !matches!(k, CardKind::Charge | CardKind::Fury | CardKind::Strike) {
                    saw_non_charge_in_unknown = true;
                }
            }
        }
        assert!(saw_non_charge_in_unknown, "sampler appears to know the true deck");
    }

    #[test]
    fn seen_cards_survive_reshuffle_in_the_observation() {
        let mut r = ChaCha8Rng::seed_from_u64(9);
        let mut s = GameState::new(&mut r);
        // First plays everything it can for several turns so a reshuffle happens.
        play_some(&mut s, &mut r, 8);
        let obs = s.observe(Player::Second);
        let o = &obs.opponent;
        let revealed = o.discard_face_up.len() + o.exile.len() + o.seen_hidden.len();
        assert!(revealed > 0);
        assert!(o.seen_hidden.len() + o.discard_face_up.len() + o.exile.len() <= o.deck_size);
    }
}
