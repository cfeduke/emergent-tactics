use clap::{Parser, Subcommand};
use et_engine::{GameState, Outcome, Player};
use et_search::{Difficulty, RolloutEvaluator, search};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[derive(Parser)]
#[command(name = "et")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Play N headless AI-vs-AI games with rollout evaluators and report results.
    Arena {
        #[arg(long, default_value_t = 10)]
        games: u64,
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Arena { games, seed } => {
            let difficulty = &Difficulty::presets()[0];
            let evaluator = RolloutEvaluator { max_depth: 60 };
            let (mut first, mut second, mut draws, mut rounds) = (0, 0, 0, 0u64);
            for g in 0..games {
                let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(g));
                let mut state = GameState::new(&mut rng);
                while state.outcome().is_none() {
                    let obs = state.observe(state.to_move);
                    let legal = state.legal_actions();
                    let result = search(&obs, &legal, difficulty, &evaluator, &mut rng);
                    state.apply(&result.action, &mut rng);
                }
                rounds += state.round as u64;
                match state.outcome() {
                    Some(Outcome::Win(Player::First)) => first += 1,
                    Some(Outcome::Win(Player::Second)) => second += 1,
                    _ => draws += 1,
                }
            }
            println!("games={games} first={first} second={second} draws={draws} avg_rounds={:.1}", rounds as f64 / games as f64);
        }
    }
    Ok(())
}
