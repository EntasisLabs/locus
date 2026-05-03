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
