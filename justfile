# Task runner for the cross-language workflow. Install with `brew install just`.

default:
    @just --list

# Build every Rust crate
build:
    cargo build --workspace

# Rust + Python + web checks
test:
    cargo test --workspace
    cd training && uv run pytest -q
    cd web && npm run check

# Rebuild the Python extension after changing et-engine or et-py
py-dev:
    cd training && uv sync --group dev --reinstall-package emergent-tactics

# Run the gRPC inference service with an exported model
inference model="models/gen1.onnx":
    RUST_LOG=info cargo run --release -p et-inference -- --model {{model}}

# Run the game server
serve:
    RUST_LOG=info cargo run -p et-server

# Run the web client dev server
web:
    cd web && npm run dev

# Headless games between two controllers (random, novice, net-adept, ...)
arena first="net-adept" second="adept" games="100" model="models/gen1.onnx":
    cargo run --release -p et-cli -- arena --first {{first}} --second {{second}} --games {{games}} --model {{model}}

# Training stages
selfplay config="configs/baseline.yaml":
    cd training && uv run et-selfplay --config {{config}}

train config="configs/baseline.yaml":
    cd training && uv run et-train --config {{config}}

export config="configs/baseline.yaml":
    cd training && uv run et-export --config {{config}}

# Rank presets with the newest exported model
arena-stage config="configs/baseline.yaml":
    cd training && uv run et-arena --config {{config}}

# One full generation with the newest model: self-play, train, export, arena
generation config="configs/baseline.yaml":
    cd training && uv run et-selfplay --config {{config}} --evaluator network --model latest
    cd training && uv run et-train --config {{config}}
    cd training && uv run et-export --config {{config}}
    cd training && uv run et-arena --config {{config}}
