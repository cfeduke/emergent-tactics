//! Python bindings. Self-play in `training/` drives the Rust engine and
//! encoder through this module so the rules and feature layout are never
//! duplicated in Python.

use et_engine::{GameState, Outcome, Player, encode};
use et_model::{Batcher, OnnxModel};
use et_search::{Controller, Difficulty, Evaluator, EvaluatorKind, NetworkEvaluator, RolloutEvaluator, SelfPlayConfig, SelfPlayStats, Sample, arena, search, self_play};
use std::sync::Arc;
use std::time::Duration;
use pyo3::types::PyBytes;
use pyo3::prelude::*;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

#[pyclass(name = "Game")]
struct PyGame {
    state: GameState,
    rng: ChaCha8Rng,
}

#[pymethods]
impl PyGame {
    #[new]
    #[pyo3(signature = (seed = 0))]
    fn new(seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        Self { state: GameState::new(&mut rng), rng }
    }

    fn turn(&self) -> u32 {
        self.state.turn
    }

    fn round(&self) -> u32 {
        self.state.round
    }

    /// 0 for the first player, 1 for the second.
    fn to_move(&self) -> u8 {
        self.state.to_move.index() as u8
    }

    fn legal_actions(&self) -> Vec<usize> {
        self.state.legal_actions().iter().map(encode::action_index).collect()
    }

    fn apply(&mut self, action_index: usize) -> PyResult<()> {
        let action = encode::action_from_index(action_index)
            .filter(|a| self.state.is_legal(a))
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("illegal action"))?;
        self.state.apply(&action, &mut self.rng);
        Ok(())
    }

    fn is_over(&self) -> bool {
        self.state.outcome().is_some()
    }

    /// +1 if the first player won, -1 if the second, 0 for a draw; None if ongoing.
    fn outcome(&self) -> Option<f32> {
        self.state.outcome().map(|o| match o {
            Outcome::Win(Player::First) => 1.0,
            Outcome::Win(Player::Second) => -1.0,
            Outcome::Draw => 0.0,
        })
    }

    /// Feature vector for the player to move.
    fn encode(&self) -> Vec<f32> {
        encode::encode_observation(&self.state.observe(self.state.to_move))
    }

    /// Run ISMCTS with rollout evaluation for the player to move. Returns
    /// (chosen action index, root value, [(action index, prior, visits, value)]).
    /// Self-play uses the visit distribution as the policy training target.
    #[pyo3(signature = (iterations = 200, determinizations = 10, exploration = 1.4, temperature = 1.0, rollout_depth = 60))]
    #[allow(clippy::type_complexity)]
    fn search(&mut self, iterations: u32, determinizations: u32, exploration: f32, temperature: f32, rollout_depth: u32) -> (usize, f32, Vec<(usize, f32, u32, f32)>) {
        let d = Difficulty { name: "py".into(), iterations, determinizations, temperature, exploration, root_noise: None, evaluator: EvaluatorKind::Rollout };
        let obs = self.state.observe(self.state.to_move);
        let legal = self.state.legal_actions();
        let res = search(&obs, &legal, &d, &RolloutEvaluator { max_depth: rollout_depth }, &mut self.rng);
        let cands = res.stats.candidates.iter().map(|c| (encode::action_index(&c.action), c.prior, c.visits, c.value)).collect();
        (encode::action_index(&res.action), res.stats.root_value, cands)
    }

    fn observation_json(&self) -> String {
        serde_json::to_string(&self.state.observe(self.state.to_move)).expect("serializable")
    }
}

fn f32_bytes<'py>(py: Python<'py>, v: &[f32]) -> Bound<'py, PyBytes> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for x in v {
        out.extend_from_slice(&x.to_le_bytes());
    }
    PyBytes::new(py, &out)
}

fn load_model(path: &str, intra_threads: usize) -> PyResult<Arc<OnnxModel>> {
    let m = OnnxModel::load(std::path::Path::new(path), intra_threads).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let ours = et_engine::library_fingerprint_hex();
    if m.info().library_fingerprint != ours {
        return Err(pyo3::exceptions::PyValueError::new_err(format!("model fingerprint {} != engine {ours}", m.info().library_fingerprint)));
    }
    Ok(Arc::new(m))
}

#[allow(clippy::too_many_arguments)]
fn selfplay_config(games: u32, seed: u64, iterations: u32, determinizations: u32, exploration: f32, temperature: f32, temperature_turns: u32, root_noise_alpha: f32, root_noise_weight: f32, threads: usize) -> SelfPlayConfig {
    SelfPlayConfig {
        games,
        seed,
        iterations,
        determinizations,
        exploration,
        temperature,
        temperature_turns,
        root_noise: if root_noise_weight > 0.0 { Some((root_noise_alpha, root_noise_weight)) } else { None },
        threads,
    }
}

/// Play games in parallel with rollout-evaluated ISMCTS on both sides and
/// return training samples as little-endian float32 buffers:
/// features (n × FEATURE_LEN), policy (n × ACTION_SPACE), legal mask
/// (n × ACTION_SPACE), value (n), plus per-sample (player, game, turn) and
/// a stats dict. The GIL is released while playing.
#[pyfunction]
#[pyo3(signature = (games, seed = 0, iterations = 200, determinizations = 10, exploration = 1.4, temperature = 1.0, temperature_turns = 1000, root_noise_alpha = 0.5, root_noise_weight = 0.25, rollout_depth = 60, threads = 0))]
#[allow(clippy::too_many_arguments)]
fn self_play_rollout<'py>(
    py: Python<'py>,
    games: u32,
    seed: u64,
    iterations: u32,
    determinizations: u32,
    exploration: f32,
    temperature: f32,
    temperature_turns: u32,
    root_noise_alpha: f32,
    root_noise_weight: f32,
    rollout_depth: u32,
    threads: usize,
) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let config = selfplay_config(games, seed, iterations, determinizations, exploration, temperature, temperature_turns, root_noise_alpha, root_noise_weight, threads);
    let (samples, stats) = py.detach(|| self_play(&config, || RolloutEvaluator { max_depth: rollout_depth }));
    pack_samples(py, samples, stats, None)
}

/// Same as `self_play_rollout` but leaves are evaluated by the ONNX model
/// at `model`, batched across all worker threads. The stats dict also
/// reports `batches` and `requests` so the batching efficiency is visible.
#[pyfunction]
#[pyo3(signature = (model, games, seed = 0, iterations = 200, determinizations = 10, exploration = 1.4, temperature = 1.0, temperature_turns = 1000, root_noise_alpha = 0.5, root_noise_weight = 0.25, threads = 0, max_batch = 64, max_wait_us = 500, intra_threads = 2))]
#[allow(clippy::too_many_arguments)]
fn self_play_network<'py>(
    py: Python<'py>,
    model: &str,
    games: u32,
    seed: u64,
    iterations: u32,
    determinizations: u32,
    exploration: f32,
    temperature: f32,
    temperature_turns: u32,
    root_noise_alpha: f32,
    root_noise_weight: f32,
    threads: usize,
    max_batch: usize,
    max_wait_us: u64,
    intra_threads: usize,
) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let config = selfplay_config(games, seed, iterations, determinizations, exploration, temperature, temperature_turns, root_noise_alpha, root_noise_weight, threads);
    let m = load_model(model, intra_threads)?;
    let batcher = Batcher::spawn(m, max_batch, Duration::from_micros(max_wait_us));
    let b2 = batcher.clone();
    let (samples, stats) = py.detach(|| self_play(&config, || NetworkEvaluator::new(b2.clone()).expect("dims checked at load")));
    let bs = batcher.stats();
    pack_samples(py, samples, stats, Some((bs.requests, bs.batches)))
}

/// Headless matches: controllers are "random" or a preset name; net-*
/// presets need `model`. Returns a stats dict.
#[pyfunction]
#[pyo3(signature = (first, second, games, seed = 0, model = None, threads = 0))]
fn arena_match<'py>(py: Python<'py>, first: &str, second: &str, games: u32, seed: u64, model: Option<&str>, threads: usize) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let side = |name: &str| -> PyResult<Option<Difficulty>> {
        if name == "random" {
            return Ok(None);
        }
        Difficulty::by_name(name).map(Some).ok_or_else(|| pyo3::exceptions::PyValueError::new_err(format!("unknown controller {name}")))
    };
    let (a, b) = (side(first)?, side(second)?);
    let needs_net = [&a, &b].iter().any(|s| matches!(s, Some(d) if d.evaluator == EvaluatorKind::Network));
    let rollout = RolloutEvaluator { max_depth: 60 };
    let net = if needs_net {
        let path = model.ok_or_else(|| pyo3::exceptions::PyValueError::new_err("net-* presets need model"))?;
        Some(NetworkEvaluator::new(Batcher::spawn(load_model(path, 2)?, 64, Duration::from_micros(500))).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?)
    } else {
        None
    };
    let ctl = |s: &Option<Difficulty>| -> Controller<'_> {
        match s {
            None => Controller::Random,
            Some(d) => {
                let evaluator: &dyn Evaluator = if d.evaluator == EvaluatorKind::Network { net.as_ref().expect("net") } else { &rollout };
                Controller::Search { difficulty: d.clone(), evaluator }
            }
        }
    };
    let (ca, cb) = (ctl(&a), ctl(&b));
    let stats = py.detach(|| arena(&ca, &cb, games, seed, threads));
    let d = pyo3::types::PyDict::new(py);
    d.set_item("games", stats.games)?;
    d.set_item("first_wins", stats.a_wins)?;
    d.set_item("second_wins", stats.b_wins)?;
    d.set_item("draws", stats.draws)?;
    d.set_item("total_rounds", stats.total_rounds)?;
    d.set_item("elapsed_secs", stats.elapsed_secs)?;
    Ok(d)
}

fn pack_samples<'py>(py: Python<'py>, samples: Vec<Sample>, stats: SelfPlayStats, batching: Option<(u64, u64)>) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let n = samples.len();
    let mut features = Vec::with_capacity(n * encode::FEATURE_LEN);
    let mut policy = Vec::with_capacity(n * encode::ACTION_SPACE);
    let mut legal = Vec::with_capacity(n * encode::ACTION_SPACE);
    let mut value = Vec::with_capacity(n);
    let mut meta: Vec<(u8, u32, u32)> = Vec::with_capacity(n);
    for s in &samples {
        features.extend_from_slice(&s.features);
        policy.extend_from_slice(&s.policy);
        legal.extend_from_slice(&s.legal);
        value.push(s.value);
        meta.push((s.player.index() as u8, s.game, s.turn));
    }
    let d = pyo3::types::PyDict::new(py);
    d.set_item("n", n)?;
    d.set_item("features", f32_bytes(py, &features))?;
    d.set_item("policy", f32_bytes(py, &policy))?;
    d.set_item("legal", f32_bytes(py, &legal))?;
    d.set_item("value", f32_bytes(py, &value))?;
    d.set_item("meta", meta)?;
    let st = pyo3::types::PyDict::new(py);
    st.set_item("games", stats.games)?;
    st.set_item("samples", stats.samples)?;
    st.set_item("first_wins", stats.first_wins)?;
    st.set_item("second_wins", stats.second_wins)?;
    st.set_item("draws", stats.draws)?;
    st.set_item("total_rounds", stats.total_rounds)?;
    st.set_item("elapsed_secs", stats.elapsed_secs)?;
    if let Some((requests, batches)) = batching {
        st.set_item("requests", requests)?;
        st.set_item("batches", batches)?;
    }
    d.set_item("stats", st)?;
    Ok(d)
}

#[pymodule]
fn emergent_tactics(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyGame>()?;
    m.add_function(wrap_pyfunction!(self_play_rollout, m)?)?;
    m.add_function(wrap_pyfunction!(self_play_network, m)?)?;
    m.add_function(wrap_pyfunction!(arena_match, m)?)?;
    m.add("FEATURE_LEN", encode::FEATURE_LEN)?;
    m.add("ACTION_SPACE", encode::ACTION_SPACE)?;
    m.add("ACTION_NAMES", action_names())?;
    m.add("LIBRARY_FINGERPRINT", et_engine::library_fingerprint_hex())?;
    m.add("STATUS_NAMES", et_engine::StatusKind::ALL.iter().map(|k| k.def().name.to_string()).collect::<Vec<_>>())?;
    m.add("ARCHETYPE_NAMES", et_engine::Archetype::ALL.iter().map(|a| a.def().name.to_string()).collect::<Vec<_>>())?;
    Ok(())
}

fn action_names() -> Vec<String> {
    (0..encode::ACTION_SPACE)
        .map(|i| match encode::action_from_index(i) {
            Some(et_engine::Action::Play(k)) => k.def().name.to_string(),
            _ => "End Turn".to_string(),
        })
        .collect()
}
