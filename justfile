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

# Run the gRPC inference service (optionally with a model)
inference model="":
    RUST_LOG=info cargo run -p et-inference -- {{ if model != "" { "--model " + model } else { "" } }}

# Run the game server
serve:
    RUST_LOG=info cargo run -p et-server

# Run the web client dev server
web:
    cd web && npm run dev

# Headless AI-vs-AI games
arena games="10":
    cargo run -p et-cli -- arena --games {{games}}

# Training stages
selfplay config="configs/baseline.yaml":
    cd training && uv run et-selfplay --config {{config}}

train config="configs/baseline.yaml":
    cd training && uv run et-train --config {{config}}

export config="configs/baseline.yaml":
    cd training && uv run et-export --config {{config}}
