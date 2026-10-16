# ADR 0001: The Rust engine is the single source of truth for rules and encoding

Status: accepted · Date: 2026-10-16

## Context

Self-play training runs in Python with PyTorch, but the game is served from
Rust. Any divergence between how training and serving simulate the game, or
how they lay out the input features, silently corrupts the model.

## Decision

Game rules and the observation-to-tensor encoder live only in `et-engine`.
Python reaches them through `et-py` (PyO3). Python never reimplements a rule
or a feature index; `FEATURE_LEN` and `ACTION_SPACE` are imported from the
extension module.

## Consequences

- Changing the rules or encoder requires rebuilding the extension
  (`just py-dev`), which is a deliberate friction.
- A parity test comparing PyTorch and ONNX Runtime outputs on a fixed batch is
  mandatory before any model is served.
