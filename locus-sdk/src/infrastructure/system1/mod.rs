//! System 1 deciders a host can attach to the memory reflex.
//!
//! `HeuristicSystem1` answers the memory catalog with lexical cues so the reflex
//! runs with no checkpoint. `HttpSystem1` posts the same questions to a
//! Laya, `sys1`, or Jev-compatible `/v1/systemone` server.

mod heuristic;

pub use heuristic::HeuristicSystem1;

#[cfg(feature = "http-providers")]
mod http;
#[cfg(feature = "http-providers")]
pub use http::HttpSystem1;
