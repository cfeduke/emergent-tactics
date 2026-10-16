//! Tensor encoding of an [`Observation`] for the policy/value network, and
//! the fixed action index. Single source of truth for both; Python imports
//! these through `et-py`.

use crate::cards::CardKind;
use crate::state::Action;
use crate::Observation;

const KINDS: usize = CardKind::ALL.len();

/// Per-entity public block: life, block, resource, limit, bonus, vulnerable,
/// fury, stand_ground, hand, draw, face-down discard, then face-up discard
/// counts and exile counts per kind.
const PUBLIC_LEN: usize = 11 + 2 * KINDS;

/// Viewer's hand counts and face-down discard counts per kind, plus
/// round and to-move flag.
pub const FEATURE_LEN: usize = 2 * PUBLIC_LEN + 2 * KINDS + 2;

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

fn push_public(out: &mut Vec<f32>, e: &crate::observation::PublicEntity) {
    out.extend_from_slice(&[
        e.lifepool as f32 / 20.0,
        e.block as f32 / 10.0,
        e.resource as f32 / 5.0,
        e.resource_limit as f32 / 5.0,
        e.resource_bonus as f32 / 5.0,
        e.vulnerable as f32 / 4.0,
        if e.fury { 1.0 } else { 0.0 },
        e.stand_ground_pending as f32,
        e.hand_count as f32 / 10.0,
        e.draw_count as f32 / 10.0,
        e.discard_face_down_count as f32 / 10.0,
    ]);
    out.extend_from_slice(&counts(&e.discard_face_up));
    out.extend_from_slice(&counts(&e.exile));
}

pub fn encode_observation(obs: &Observation) -> Vec<f32> {
    let mut out = Vec::with_capacity(FEATURE_LEN);
    push_public(&mut out, &obs.me);
    push_public(&mut out, &obs.opponent);
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
}
