mod cli;
mod config;
mod error;
mod models;
mod routes;
mod server;

pub use config::config;
pub use error::{Error, Result};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cli::Cli::run().await
}
