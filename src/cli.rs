use clap::{Args, Parser, Subcommand};

use crate::server;

#[derive(Debug, Parser)]
#[command(version, max_term_width = 120)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    pub async fn run() -> anyhow::Result<()> {
        let cli = Self::parse();

        cli.command.run().await
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    Server(Server),
}

impl Command {
    async fn run(&self) -> anyhow::Result<()> {
        match self {
            Command::Server(cmd) => cmd.run().await,
        }
    }
}

#[derive(Debug, Clone, Args)]
pub struct Server {}

impl Server {
    async fn run(&self) -> anyhow::Result<()> {
        server::serve().await
    }
}
