//! SurrealDB connection adapter implementing [`locus_core_rs::SurrealDbClient`].
//!
//! Native hosts enable the `native` feature for embedded KV and HTTP transports.
//! Browser or WASM consumers should use `default-features = false` with the `wasm`
//! feature and connect to:
//! - `indxdb://<name>` for persistent IndexedDB storage
//! - `mem://` for in-memory embedded storage
//! - `ws://` / `wss://` for remote WebSocket servers

mod client;
mod endpoint;

pub use client::RuntimeSurrealDbClient;
pub use endpoint::{effective_use_remote, is_embedded_endpoint, is_remote_endpoint};
