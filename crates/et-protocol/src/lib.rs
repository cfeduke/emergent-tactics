//! Wire types for every boundary in the system.
//!
//! * [`ws`]: JSON messages between the web client and the game server.
//! * [`inference`]: gRPC service between the game server and the inference
//!   service, generated from `proto/inference.proto`.

pub mod ws;

pub mod inference {
    tonic::include_proto!("emergent_tactics.inference.v1");
}
