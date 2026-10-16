//! Loads an exported policy/value model and evaluates feature batches.
//! Game-agnostic: dimensions and the library fingerprint come from the
//! ONNX metadata written by `et-export`.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, SyncSender, channel, sync_channel};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use ort::environment::Environment;
use ort::session::{Session, builder::GraphOptimizationLevel};
use ort::value::Tensor;

fn environment() -> anyhow::Result<&'static Environment> {
    static ENV: OnceLock<Environment> = OnceLock::new();
    if let Some(e) = ENV.get() {
        return Ok(e);
    }
    let env = ort::init().with_name("emergent-tactics").build()?;
    let _ = ENV.set(env);
    Ok(ENV.get().expect("set"))
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub library_fingerprint: String,
    pub feature_len: usize,
    pub action_space: usize,
}

pub struct OnnxModel {
    session: Mutex<Session>,
    info: ModelInfo,
}

impl OnnxModel {
    pub fn load(path: &Path, intra_threads: usize) -> anyhow::Result<Self> {
        let env = environment()?;
        let session = Session::builder(env)?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .with_intra_threads(intra_threads.max(1))
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .commit_from_file(path)?;
        let info = {
            let meta = session.metadata()?;
            let get = |k: &str| meta.custom(k).unwrap_or_default();
            ModelInfo {
                name: get("model_name"),
                library_fingerprint: get("library_fingerprint"),
                feature_len: get("feature_len").parse().unwrap_or(0),
                action_space: get("action_space").parse().unwrap_or(0),
            }
        };
        anyhow::ensure!(info.feature_len > 0 && info.action_space > 0, "model {} lacks feature_len/action_space metadata", path.display());
        Ok(Self { session: Mutex::new(session), info })
    }

    pub fn info(&self) -> &ModelInfo {
        &self.info
    }

    /// Evaluate `n` rows of `feature_len` features. Returns flat policy
    /// logits (n × action_space) and values (n).
    pub fn evaluate(&self, features: &[f32], n: usize) -> anyhow::Result<(Vec<f32>, Vec<f32>)> {
        anyhow::ensure!(features.len() == n * self.info.feature_len, "expected {} features, got {}", n * self.info.feature_len, features.len());
        if n == 0 {
            return Ok((Vec::new(), Vec::new()));
        }
        let input = Tensor::from_array(([n, self.info.feature_len], features.to_vec()))?;
        let mut session = self.session.lock().expect("session lock");
        let outputs = session.run(ort::inputs!["features" => input])?;
        let (_, policy) = outputs["policy"].try_extract_tensor::<f32>()?;
        let (_, value) = outputs["value"].try_extract_tensor::<f32>()?;
        Ok((policy.to_vec(), value.to_vec()))
    }
}

struct Request {
    features: Vec<f32>,
    reply: SyncSender<(Vec<f32>, f32)>,
}

#[derive(Debug, Default, Clone)]
pub struct BatcherStats {
    pub requests: u64,
    pub batches: u64,
}

/// Merges evaluation requests from many threads into batched model calls.
/// Requests are gathered until `max_batch` are waiting or `max_wait` has
/// passed since the first one arrived, then evaluated together.
#[derive(Clone)]
pub struct Batcher {
    tx: Sender<Request>,
    requests: Arc<AtomicU64>,
    batches: Arc<AtomicU64>,
    feature_len: usize,
    action_space: usize,
}

impl Batcher {
    pub fn spawn(model: Arc<OnnxModel>, max_batch: usize, max_wait: Duration) -> Self {
        let (tx, rx) = channel::<Request>();
        let requests = Arc::new(AtomicU64::new(0));
        let batches = Arc::new(AtomicU64::new(0));
        let (req_c, bat_c) = (requests.clone(), batches.clone());
        let info = model.info().clone();
        std::thread::Builder::new()
            .name("et-batcher".into())
            .spawn(move || run_batcher(model, rx, max_batch, max_wait, req_c, bat_c))
            .expect("spawn batcher");
        Self { tx, requests, batches, feature_len: info.feature_len, action_space: info.action_space }
    }

    pub fn feature_len(&self) -> usize {
        self.feature_len
    }

    pub fn action_space(&self) -> usize {
        self.action_space
    }

    /// Blocks until the model has evaluated this row.
    pub fn evaluate(&self, features: Vec<f32>) -> (Vec<f32>, f32) {
        let (reply, wait) = sync_channel(1);
        self.tx.send(Request { features, reply }).expect("batcher alive");
        wait.recv().expect("batcher reply")
    }

    pub fn stats(&self) -> BatcherStats {
        BatcherStats { requests: self.requests.load(Ordering::Relaxed), batches: self.batches.load(Ordering::Relaxed) }
    }
}

fn run_batcher(model: Arc<OnnxModel>, rx: Receiver<Request>, max_batch: usize, max_wait: Duration, requests: Arc<AtomicU64>, batches: Arc<AtomicU64>) {
    let info = model.info().clone();
    while let Ok(first) = rx.recv() {
        let mut pending = vec![first];
        let deadline = Instant::now() + max_wait;
        while pending.len() < max_batch {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            match rx.recv_timeout(deadline - now) {
                Ok(r) => pending.push(r),
                Err(_) => break,
            }
        }
        let n = pending.len();
        let mut features = Vec::with_capacity(n * info.feature_len);
        for r in &pending {
            features.extend_from_slice(&r.features);
        }
        match model.evaluate(&features, n) {
            Ok((policy, value)) => {
                for (i, r) in pending.into_iter().enumerate() {
                    let row = policy[i * info.action_space..(i + 1) * info.action_space].to_vec();
                    let _ = r.reply.send((row, value[i]));
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "model evaluation failed");
                for r in pending {
                    let _ = r.reply.send((vec![0.0; info.action_space], 0.0));
                }
            }
        }
        requests.fetch_add(n as u64, Ordering::Relaxed);
        batches.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen1() -> Option<std::path::PathBuf> {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../models/gen1.onnx");
        p.exists().then_some(p)
    }

    #[test]
    fn loads_metadata_and_evaluates_batches() {
        let Some(p) = gen1() else { return };
        let m = OnnxModel::load(&p, 1).unwrap();
        assert_eq!(m.info().library_fingerprint.len(), 16);
        let n = 5;
        let x = vec![0.1f32; n * m.info().feature_len];
        let (policy, value) = m.evaluate(&x, n).unwrap();
        assert_eq!(policy.len(), n * m.info().action_space);
        assert_eq!(value.len(), n);
        assert!(value.iter().all(|v| (-1.0..=1.0).contains(v)));
    }

    #[test]
    fn batcher_merges_concurrent_requests() {
        let Some(p) = gen1() else { return };
        let m = Arc::new(OnnxModel::load(&p, 1).unwrap());
        let b = Batcher::spawn(m.clone(), 64, Duration::from_millis(5));
        let fl = m.info().feature_len;
        let handles: Vec<_> = (0..32)
            .map(|i| {
                let b = b.clone();
                std::thread::spawn(move || b.evaluate(vec![i as f32 / 32.0; fl]))
            })
            .collect();
        for h in handles {
            let (p, _) = h.join().unwrap();
            assert_eq!(p.len(), m.info().action_space);
        }
        let s = b.stats();
        assert_eq!(s.requests, 32);
        assert!(s.batches < 32, "expected batching, got {} batches", s.batches);
    }
}
