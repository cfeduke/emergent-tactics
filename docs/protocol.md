# Wire protocol

## Web client ↔ game server (JSON over WebSocket)

Messages are tagged enums; the `type` field selects the variant. Definitions
live in `crates/et-protocol/src/ws.rs` and are the authority.

Client → server: `new_game`, `spectate`, `act`.

Server → client: `game_started`, `state`, `thinking`, `acted`, `game_over`,
`error`.

`thinking` carries per-action visit counts during search so the client can
render what the AI is considering.

## Game server ↔ inference service (gRPC)

See `crates/et-protocol/proto/inference.proto`. `Evaluate` takes a flat
row-major feature batch and returns flat policy logits and one value per row.
`Info` reports the loaded checkpoint and expected dimensions so the server can
refuse a mismatched model at startup.
