//! Infrastructure adapters and provider integrations.
//!
//! Contains embedding providers, registry wiring, and STTP-native bridging
//! used by higher-level SDK services.

pub mod embeddings;
pub mod genai_adapter;
pub mod registry;
pub mod sttp_native;
