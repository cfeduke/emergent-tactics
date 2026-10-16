# ADR 0002: Inference runs in a separate service, reached over gRPC

Status: accepted · Date: 2026-10-16

## Context

Model evaluation could run in-process in `et-server` with the `ort` crate.
The project's purpose includes learning the architecture a production game
would use, where inference scales independently of game sessions and may run
on different hardware.

## Decision

`et-inference` is a standalone binary exposing an `Inference` gRPC service
(`crates/et-protocol/proto/inference.proto`). `et-server` calls it through a
tonic client wrapped in an `et-search::Evaluator`. Requests carry flat float
batches so the search can amortise network round trips.

The web-to-server boundary stays JSON over WebSocket: it is low-volume and
human inspection in browser dev tools is worth more than compactness there.

## Consequences

- Protobuf codegen happens at build time via `protox`, so no `protoc`
  install is required.
- Search latency now includes a network hop; batching leaves per iteration
  becomes a real design concern rather than a hypothetical one.
- The service is game-agnostic and can serve any model with matching
  feature/action dimensions, which is the correct boundary for swapping
  checkpoints to change difficulty.
