use std::{io::Write, net::TcpStream};

use clap::{Parser, Subcommand};

use shared::Command;

#[derive(Parser)]
#[command(
    version = "1.0",
    about = "CLI to interact with the Web Crawler Service"
)]
struct Cli {
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Subcommand)]
enum CliCommand {
    /// Starts crawling a given URL
    Start {
        /// The URL to start crawling
        url: String,
    },
    /// Stops crawling a given URL
    Stop {
        /// The URL to stop crawling
        url: String,
    },
    /// Lists all crawled URLs
    List,
}

impl CliCommand {
    fn to_protocol(&self) -> Command {
        match self {
            CliCommand::Start { url } => Command::Start(url.clone()),
            CliCommand::Stop { url } => Command::Stop(url.clone()),
            CliCommand::List => Command::List,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let cli = Cli::parse();
    let command = cli.command.to_protocol();

    // Connect to the service over TCP
    let mut stream = TcpStream::connect("127.0.0.1:8080")?;
    let encoded: Vec<u8> = bincode::serialize(&command)?;
    stream.write_all(&encoded)?;

    Ok(())
}
