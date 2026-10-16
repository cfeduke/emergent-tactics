//! Parallel self-play: play many games with the search on both sides and
//! collect (observation, visit distribution, outcome) samples for training.

use et_engine::{Action, GameState, Outcome, Player, encode};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{Difficulty, Evaluator, search};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfPlayConfig {
    pub games: u32,
    pub seed: u64,
    pub iterations: u32,
    pub determinizations: u32,
    pub exploration: f32,
    /// Sampling temperature for the first `temperature_turns` entity turns
    /// of each game; afterwards the most visited action is played.
    pub temperature: f32,
    pub temperature_turns: u32,
    /// Dirichlet noise mixed into root priors for exploration: (alpha, weight).
    pub root_noise: Option<(f32, f32)>,
    /// 0 means use every core.
    pub threads: usize,
}

/// One training sample. `policy` is the normalised root visit distribution
/// over the full action space (zeros for illegal actions); `legal` masks
/// the actions that were available.
#[derive(Debug, Clone)]
pub struct Sample {
    pub features: Vec<f32>,
    pub policy: Vec<f32>,
    pub legal: Vec<f32>,
    /// Game result from the acting player's perspective: +1 win, -1 loss, 0 draw.
    pub value: f32,
    pub player: Player,
    pub game: u32,
    pub turn: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SelfPlayStats {
    pub games: u32,
    pub samples: usize,
    pub first_wins: u32,
    pub second_wins: u32,
    pub draws: u32,
    pub total_rounds: u64,
    pub elapsed_secs: f64,
}

/// Play `config.games` games in parallel. `make_evaluator` is called once
/// per worker thread so evaluators need not be `Sync`.
pub fn self_play<E, F>(config: &SelfPlayConfig, make_evaluator: F) -> (Vec<Sample>, SelfPlayStats)
where
    E: Evaluator,
    F: Fn() -> E + Sync + Send,
{
    let started = std::time::Instant::now();
    let pool = rayon::ThreadPoolBuilder::new().num_threads(config.threads).build().expect("thread pool");
    let per_game: Vec<(Vec<Sample>, Outcome, u32)> = pool.install(|| {
        (0..config.games)
            .into_par_iter()
            .map_init(
                || make_evaluator(),
                |evaluator, g| play_one(config, g, evaluator),
            )
            .collect()
    });
    let mut stats = SelfPlayStats { games: config.games, ..Default::default() };
    let mut samples = Vec::new();
    for (s, outcome, rounds) in per_game {
        match outcome {
            Outcome::Win(Player::First) => stats.first_wins += 1,
            Outcome::Win(Player::Second) => stats.second_wins += 1,
            Outcome::Draw => stats.draws += 1,
        }
        stats.total_rounds += rounds as u64;
        samples.extend(s);
    }
    stats.samples = samples.len();
    stats.elapsed_secs = started.elapsed().as_secs_f64();
    (samples, stats)
}

fn play_one(config: &SelfPlayConfig, game: u32, evaluator: &dyn Evaluator) -> (Vec<Sample>, Outcome, u32) {
    let mut rng = ChaCha8Rng::seed_from_u64(config.seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(game as u64));
    let mut state = GameState::new(&mut rng);
    let mut pending: Vec<Sample> = Vec::new();
    while state.outcome().is_none() {
        let player = state.to_move;
        let obs = state.observe(player);
        let legal = state.legal_actions();
        let temperature = if state.turn < config.temperature_turns { config.temperature } else { 0.0 };
        let difficulty = Difficulty {
            name: "selfplay".into(),
            iterations: config.iterations,
            determinizations: config.determinizations,
            temperature,
            exploration: config.exploration,
            root_noise: config.root_noise,
        };
        let result = search(&obs, &legal, &difficulty, evaluator, &mut rng);

        let mut policy = vec![0.0f32; encode::ACTION_SPACE];
        let mut legal_mask = vec![0.0f32; encode::ACTION_SPACE];
        let total: u32 = result.stats.candidates.iter().map(|c| c.visits).sum();
        for c in &result.stats.candidates {
            let i = encode::action_index(&c.action);
            legal_mask[i] = 1.0;
            if total > 0 {
                policy[i] = c.visits as f32 / total as f32;
            }
        }
        for a in &legal {
            legal_mask[encode::action_index(a)] = 1.0;
        }
        pending.push(Sample { features: encode::encode_observation(&obs), policy, legal: legal_mask, value: 0.0, player, game, turn: state.turn });

        let action: Action = result.action;
        state.apply(&action, &mut rng);
    }
    let outcome = state.outcome().expect("terminal");
    for s in &mut pending {
        s.value = crate::evaluator::outcome_value(outcome, s.player);
    }
    (pending, outcome, state.round)
}
