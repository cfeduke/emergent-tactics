# Emergent Tactics

A two-player card battler built to learn, hands-on, how Information Set Monte
Carlo Tree Search (ISMCTS) combined with a trained policy/value network can
produce AI opponents at different difficulty levels.

The game is playable (human vs. AI, or AI vs. AI to spectate) but the point is
the pipeline: self-play, training in PyTorch, exporting to ONNX, and serving
the model to a Rust search from a separate inference service.

## Architecture

```
web (SvelteKit) --JSON/WebSocket--> et-server --gRPC--> et-inference (ONNX)
                                        |
                                    et-search (ISMCTS)
                                        |
                                    et-engine (rules)
                                        ^
                       training/ (PyTorch) --via et-py (PyO3)
```

| Path | Purpose |
|------|---------|
| `crates/et-engine` | Pure game rules, observations, and the feature encoder. Single source of truth. |
| `crates/et-search` | ISMCTS with a pluggable `Evaluator` (random rollouts or network). Difficulty presets. |
| `crates/et-protocol` | JSON message types for the web client, gRPC service definition for inference. |
| `crates/et-model` | ONNX model loading, metadata checks, and a batching thread that merges leaf evaluations from many searches into one model call. |
| `crates/et-inference` | Game-agnostic gRPC service that evaluates feature batches with an ONNX model. |
| `crates/et-server` | Game sessions over WebSocket; drives the search for AI turns. |
| `crates/et-cli` | Headless arena, benchmarks, and calibration. |
| `crates/et-py` | PyO3 bindings so self-play uses the Rust engine. |
| `training/` | Self-play, training, export, and arena stages. |
| `web/` | Browser client with a search-statistics debug panel. |
| `docs/adr/` | Architecture decision records. |

## Prerequisites

- Rust (stable), `cargo`
- Python 3.12 via [uv](https://docs.astral.sh/uv/)
- Node 22+
- [`just`](https://github.com/casey/just) (optional task runner)

## Getting started

```sh
cargo build --workspace
cd training && uv sync --group dev && uv run pytest
cd web && npm install && npm run check
```

See the `justfile` for the full set of tasks.

## Playing

Run the game server and the web client in two terminals, then open
http://localhost:5173. Vite proxies `/ws` to the server.

```sh
RUST_LOG=info cargo run -p et-server
cd web && npm run dev
```

Pick a difficulty and seat to play against the AI, or choose two
difficulties and watch them play. The log panel shows every AI decision:
the candidate actions with their priors, visit counts and values, plus a
summary of what each action changed.

## Training loop

All stages read `training/configs/baseline.yaml` and run from `training/`.

1. `uv run et-selfplay` plays games in parallel in Rust (all cores, GIL
   released) with ISMCTS on both sides and writes
   `data/gen<N>/samples.npz`: encoded observations, root visit
   distributions, legal masks and game results. Generation 1 uses random
   rollouts (`--evaluator rollout`); later generations use the newest
   exported model (`--evaluator network --model latest`), with leaf
   evaluations batched across all worker threads.
2. `uv run et-train` fits the policy/value network on the most recent
   generations. `train.device` selects `cpu`, `mps` (Metal) or `auto`.
   Writes `models/gen<N>.pt`.
3. `uv run et-export` writes `models/gen<N>.onnx` with the library
   fingerprint and dimensions in the ONNX metadata, and checks ONNX Runtime
   matches PyTorch.
4. `cargo run -p et-inference -- --model models/gen<N>.onnx` serves it;
   `cargo run -p et-server` refuses a model whose fingerprint differs.
5. `uv run et-arena` ranks presets with the newest model. Presets come in
   pairs at equal search budget, `adept` (rollouts) and `net-adept`
   (network), so the value of the trained evaluator is measured directly.
6. Repeat. `just generation` runs one full loop.

Every generation directory and checkpoint records the library fingerprint,
and each stage refuses inputs produced against a different library.

## License

MIT. See [LICENSE](LICENSE).
