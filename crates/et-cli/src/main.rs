use clap::{Parser, Subcommand};
use et_engine::{Action, GameState, Outcome, Player};
use et_search::{Difficulty, RolloutEvaluator, search};
use rand::SeedableRng;
use rand::seq::IndexedRandom;
use rand_chacha::ChaCha8Rng;
use std::time::Instant;

#[derive(Parser)]
#[command(name = "et")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Play N headless games between two controllers and report results.
    /// Controllers: random, or a difficulty preset name.
    Arena {
        #[arg(long, default_value = "novice")]
        first: String,
        #[arg(long, default_value = "random")]
        second: String,
        #[arg(long, default_value_t = 10)]
        games: u64,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// Swap seats every other game so both controllers play First equally.
        #[arg(long, default_value_t = true)]
        alternate: bool,
    },
}

enum Controller {
    Random,
    Search(Difficulty),
}

fn controller(name: &str) -> anyhow::Result<Controller> {
    if name == "random" {
        return Ok(Controller::Random);
    }
    Difficulty::by_name(name).map(Controller::Search).ok_or_else(|| anyhow::anyhow!("unknown controller {name}"))
}

fn pick(c: &Controller, state: &GameState, evaluator: &RolloutEvaluator, rng: &mut ChaCha8Rng) -> Action {
    let legal = state.legal_actions();
    match c {
        Controller::Random => *legal.choose(rng).expect("legal"),
        Controller::Search(d) => search(&state.observe(state.to_move), &legal, d, evaluator, rng).action,
    }
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Arena { first, second, games, seed, alternate } => {
            let a = controller(&first)?;
            let b = controller(&second)?;
            let evaluator = RolloutEvaluator { max_depth: 60 };
            let (mut a_wins, mut b_wins, mut draws, mut rounds) = (0u64, 0u64, 0u64, 0u64);
            let started = Instant::now();
            for g in 0..games {
                let mut rng = ChaCha8Rng::seed_from_u64(seed.wrapping_add(g));
                let a_is_first = !alternate || g % 2 == 0;
                let mut state = GameState::new(&mut rng);
                while state.outcome().is_none() {
                    let c = if (state.to_move == Player::First) == a_is_first { &a } else { &b };
                    let action = pick(c, &state, &evaluator, &mut rng);
                    state.apply(&action, &mut rng);
                }
                rounds += state.round as u64;
                match state.outcome() {
                    Some(Outcome::Win(p)) => {
                        if (p == Player::First) == a_is_first { a_wins += 1 } else { b_wins += 1 }
                    }
                    _ => draws += 1,
                }
            }
            let secs = started.elapsed().as_secs_f64();
            println!(
                "{first} vs {second}: games={games} {first}={a_wins} {second}={b_wins} draws={draws} avg_rounds={:.1} ({:.1}s, {:.2}s/game)",
                rounds as f64 / games as f64,
                secs,
                secs / games as f64
            );
        }
    }
    Ok(())
}
