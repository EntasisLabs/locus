//! `locus-gateway` binary.
//!
//! Provides HTTP and gRPC surfaces for STTP memory operations by composing
//! core services with tenant-aware orchestration and provider routing.

use anyhow::Result;

mod app_state;
mod constants;
mod gateway;
mod gateway_args;
mod http_models;
mod orchestration;
mod providers;
mod surreal_client;
mod tenant;

#[tokio::main]
async fn main() -> Result<()> {
    gateway::run().await
}
