use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use et_model::{Batcher, OnnxModel};
use et_search::{Controller, Difficulty, Evaluator, EvaluatorKind, NetworkEvaluator, RolloutEvaluator, arena};

#[derive(Parser)]
#[command(name = "et")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Play N headless games between two controllers and report results.
    /// Controllers: random, or a preset (novice, adept, master, net-novice, ...).
    Arena {
        #[arg(long, default_value = "novice")]
        first: String,
        #[arg(long, default_value = "random")]
        second: String,
        #[arg(long, default_value_t = 10)]
        games: u32,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// ONNX model for net-* presets.
        #[arg(long)]
        model: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        threads: usize,
    },
    /// Print the engine's card-library fingerprint.
    Fingerprint,
}

enum Side {
    Random,
    Search(Difficulty),
}

fn side(name: &str) -> anyhow::Result<Side> {
    if name == "random" {
        return Ok(Side::Random);
    }
    Difficulty::by_name(name).map(Side::Search).ok_or_else(|| anyhow::anyhow!("unknown controller {name}"))
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().cmd {
        Cmd::Fingerprint => println!("{}", et_engine::library_fingerprint_hex()),
        Cmd::Arena { first, second, games, seed, model, threads } => {
            let (a, b) = (side(&first)?, side(&second)?);
            let needs_net = [&a, &b].iter().any(|s| matches!(s, Side::Search(d) if d.evaluator == EvaluatorKind::Network));
            let rollout = RolloutEvaluator { max_depth: 60 };
            let net: Option<NetworkEvaluator> = if needs_net {
                let path = model.ok_or_else(|| anyhow::anyhow!("net-* presets need --model"))?;
                let m = Arc::new(OnnxModel::load(&path, 1)?);
                let ours = et_engine::library_fingerprint_hex();
                anyhow::ensure!(m.info().library_fingerprint == ours, "model fingerprint {} != engine {ours}", m.info().library_fingerprint);
                Some(NetworkEvaluator::new(Batcher::spawn(m, 64, Duration::from_micros(500)))?)
            } else {
                None
            };
            let ctl = |s: &Side| -> Controller<'_> {
                match s {
                    Side::Random => Controller::Random,
                    Side::Search(d) => {
                        let evaluator: &dyn Evaluator = if d.evaluator == EvaluatorKind::Network { net.as_ref().expect("net") } else { &rollout };
                        Controller::Search { difficulty: d.clone(), evaluator }
                    }
                }
            };
            let stats = arena(&ctl(&a), &ctl(&b), games, seed, threads);
            println!(
                "{first} vs {second}: games={games} {first}={} {second}={} draws={} avg_rounds={:.1} ({:.1}s, {:.3}s/game)",
                stats.a_wins,
                stats.b_wins,
                stats.draws,
                stats.total_rounds as f64 / games as f64,
                stats.elapsed_secs,
                stats.elapsed_secs / games as f64
            );
        }
    }
    Ok(())
}
