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

1. `just selfplay` generates samples using ISMCTS with rollout evaluation.
2. `just train` fits the policy/value network.
3. `just export` writes an ONNX checkpoint to `models/`.
4. `just inference models/<ckpt>.onnx` serves it; `just serve` and `just web` to play.
5. Repeat with the network as evaluator. Each generation is a checkpoint; the
   arena stage ranks them and maps them to difficulty presets.

## License

MIT. See [LICENSE](LICENSE).
