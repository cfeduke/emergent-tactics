use clap::{Parser, Subcommand};
use et_engine::GameState;
use et_search::{Difficulty, RolloutEvaluator, search};

#[derive(Parser)]
#[command(name = "et")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Play N headless AI-vs-AI games with rollout evaluators.
    Arena {
        #[arg(long, default_value_t = 1)]
        games: u32,
        #[arg(long, default_value_t = 20)]
        max_turns: u32,
    },
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Arena { games, max_turns } => {
            let difficulty = &Difficulty::presets()[0];
            let evaluator = RolloutEvaluator { max_depth: 50 };
            for g in 0..games {
                let mut state = GameState::new();
                while state.outcome().is_none() && state.turn < max_turns {
                    let obs = state.observe(state.to_move);
                    let legal = state.legal_actions();
                    let result = search(&obs, &legal, difficulty, &evaluator);
                    state.apply(&result.action);
                }
                println!("game {g}: {:?} after {} turns", state.outcome(), state.turn);
            }
        }
    }
    Ok(())
}
