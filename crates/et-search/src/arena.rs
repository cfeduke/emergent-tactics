//! Headless matches between two controllers, in parallel, seats alternating.

use et_engine::{Action, GameState, Outcome, Player};
use rand::SeedableRng;
use rand::seq::IndexedRandom;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{Difficulty, Evaluator, search};

/// A side in the arena: random play, or a search with a given evaluator.
pub enum Controller<'a> {
    Random,
    Search { difficulty: Difficulty, evaluator: &'a dyn Evaluator },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArenaStats {
    pub games: u32,
    pub a_wins: u32,
    pub b_wins: u32,
    pub draws: u32,
    pub total_rounds: u64,
    pub elapsed_secs: f64,
}

fn pick(c: &Controller, state: &GameState, rng: &mut ChaCha8Rng) -> Action {
    let legal = state.legal_actions();
    match c {
        Controller::Random => *legal.choose(rng).expect("legal"),
        Controller::Search { difficulty, evaluator } => search(&state.observe(state.to_move), &legal, difficulty, *evaluator, rng).action,
    }
}

/// Play `games` games between `a` and `b`, alternating who sits First.
pub fn arena(a: &Controller, b: &Controller, games: u32, seed: u64, threads: usize) -> ArenaStats
where
    for<'x> Controller<'x>: Sync,
{
    let started = std::time::Instant::now();
    let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build().expect("pool");
    let results: Vec<(Option<bool>, u32)> = pool.install(|| {
        (0..games)
            .into_par_iter()
            .map(|g| {
                let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(g as u64));
                let a_first = g % 2 == 0;
                let mut state = GameState::new(&mut rng);
                while state.outcome().is_none() {
                    let c = if (state.to_move == Player::First) == a_first { a } else { b };
                    let action = pick(c, &state, &mut rng);
                    state.apply(&action, &mut rng);
                }
                let a_won = match state.outcome() {
                    Some(Outcome::Win(p)) => Some((p == Player::First) == a_first),
                    _ => None,
                };
                (a_won, state.round)
            })
            .collect()
    });
    let mut s = ArenaStats { games, ..Default::default() };
    for (a_won, rounds) in results {
        match a_won {
            Some(true) => s.a_wins += 1,
            Some(false) => s.b_wins += 1,
            None => s.draws += 1,
        }
        s.total_rounds += rounds as u64;
    }
    s.elapsed_secs = started.elapsed().as_secs_f64();
    s
}
