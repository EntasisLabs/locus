//! SurrealDB connection adapter implementing [`locus_core_rs::SurrealDbClient`].
//!
//! Native hosts enable the `native` feature for embedded KV and HTTP transports.
//! Browser or WASM consumers should use `default-features = false` with the `wasm`
//! feature and connect to a remote SurrealDB instance over WebSocket.

mod client;

pub use client::RuntimeSurrealDbClient;
