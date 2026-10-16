//! Tensor encoding of an [`Observation`] for the policy/value network, and
//! the fixed action index. Single source of truth for both; Python imports
//! these through `et-py`.
//!
//! The opponent block contains only what the viewer legitimately observes:
//! revealed cards and zone counts, never the true deck list.

use crate::cards::{Archetype, CardKind, StatusKind};
use crate::observation::PublicEntity;
use crate::state::Action;
use crate::Observation;

const KINDS: usize = CardKind::ALL.len();
const ARCHS: usize = Archetype::ALL.len();
const STATUSES: usize = StatusKind::COUNT;

/// Per-entity public block: 9 scalars, archetype one-hot, status counters,
/// then face-up discard, exile and seen-hidden counts per kind.
const PUBLIC_LEN: usize = 9 + ARCHS + STATUSES + 3 * KINDS;

/// Two public blocks, the viewer's deck, hand and face-down counts per
/// kind, plus round and to-move flag.
pub const FEATURE_LEN: usize = 2 * PUBLIC_LEN + 3 * KINDS + 2;

/// One index per card kind plus end turn.
pub const ACTION_SPACE: usize = KINDS + 1;

pub fn action_index(action: &Action) -> usize {
    match action {
        Action::Play(k) => k.index(),
        Action::EndTurn => KINDS,
    }
}

pub fn action_from_index(i: usize) -> Option<Action> {
    if i == KINDS {
        Some(Action::EndTurn)
    } else {
        CardKind::from_index(i).map(Action::Play)
    }
}

fn counts(cards: &[CardKind]) -> [f32; KINDS] {
    let mut c = [0.0; KINDS];
    for k in cards {
        c[k.index()] += 1.0;
    }
    c
}

fn push_public(out: &mut Vec<f32>, e: &PublicEntity) {
    out.extend_from_slice(&[
        e.lifepool as f32 / 20.0,
        e.block as f32 / 10.0,
        e.resource as f32 / 5.0,
        e.resource_limit as f32 / 5.0,
        e.resource_bonus as f32 / 5.0,
        e.deck_size as f32 / 20.0,
        e.hand_count as f32 / 10.0,
        e.draw_count as f32 / 10.0,
        e.discard_face_down_count as f32 / 10.0,
    ]);
    let mut arch = [0.0; ARCHS];
    arch[e.archetype.index()] = 1.0;
    out.extend_from_slice(&arch);
    let mut st = [0.0; STATUSES];
    for (k, n) in &e.statuses {
        st[k.index()] = *n as f32 / 4.0;
    }
    out.extend_from_slice(&st);
    out.extend_from_slice(&counts(&e.discard_face_up));
    out.extend_from_slice(&counts(&e.exile));
    out.extend_from_slice(&counts(&e.seen_hidden));
}

pub fn encode_observation(obs: &Observation) -> Vec<f32> {
    let mut out = Vec::with_capacity(FEATURE_LEN);
    push_public(&mut out, &obs.me);
    push_public(&mut out, &obs.opponent);
    out.extend_from_slice(&counts(&obs.deck));
    out.extend_from_slice(&counts(&obs.hand));
    out.extend_from_slice(&counts(&obs.discard_face_down));
    out.push(obs.round as f32 / 30.0);
    out.push(if obs.to_move == obs.viewer { 1.0 } else { 0.0 });
    debug_assert_eq!(out.len(), FEATURE_LEN);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GameState;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn feature_len_matches() {
        let s = GameState::new(&mut ChaCha8Rng::seed_from_u64(1));
        assert_eq!(encode_observation(&s.observe(crate::Player::First)).len(), FEATURE_LEN);
    }

    #[test]
    fn action_index_roundtrips() {
        for i in 0..ACTION_SPACE {
            assert_eq!(action_index(&action_from_index(i).unwrap()), i);
        }
    }

    #[test]
    fn fingerprint_is_stable() {
        assert_eq!(crate::cards::library_fingerprint(), crate::cards::library_fingerprint());
        assert_eq!(crate::cards::library_fingerprint_hex().len(), 16);
    }
}
