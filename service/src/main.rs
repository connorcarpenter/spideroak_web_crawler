mod crawler;
mod url_worker;
mod parser;
mod error;
mod base_url;

use std::net::SocketAddr;

use anyhow::Result;
use bincode;
use log::{info};
use tokio::{net::TcpStream, sync::{mpsc::{Receiver, Sender}, mpsc}, io::AsyncReadExt, net::TcpListener};

use shared::Command;

use crate::{error::{print_error_and_backtrace, CrawlerError}, crawler::Crawler};

#[tokio::main]
async fn main() {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting Web Crawler Daemon on 127.0.0.1:8080");

    // Channel to receive commands from client
    let (command_sender, command_receiver) = mpsc::channel::<Command>(32);

    // Setup the request reader loop
    tokio::spawn(async move {
        if let Err(err) = request_reader_loop(command_sender).await {
            print_error_and_backtrace(err);
        }
    });

    // Setup the command receiver loop
    let crawler = Crawler::new();
    tokio::spawn(async move {
        command_receiver_loop(crawler, command_receiver).await;
    });

    std::thread::park();

    info!("Shutting down...");
}

async fn request_reader_loop(command_sender: Sender<Command>) -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    loop {
        match request_accept(&listener).await {
            Ok((socket, addr)) => {
                let sender_clone = command_sender.clone();
                tokio::spawn(async move {
                    if let Err(err) = request_read(socket, addr, sender_clone).await {
                        print_error_and_backtrace(err);
                    }
                });
            }
            Err(err) => print_error_and_backtrace(err),
        }
    }
}

async fn request_accept(listener: &TcpListener) -> Result<(TcpStream, SocketAddr)> {
    let (socket, addr) = listener.accept().await?;
    Ok((socket, addr))
}

async fn request_read(mut socket: TcpStream, addr: SocketAddr, sender_clone: Sender<Command>) -> Result<()> {
    let mut buffer = [0; 1024];
    let bytes_number = socket.read(&mut buffer).await?;
    info!("Received TCP message from: {:?}", addr);

    // Deserialize command from received bytes using bincode
    let command = bincode::deserialize::<Command>(&buffer[..bytes_number])?;

    // Send command to the command handler
    // info!("Sending to command channel: {:?}", command);
    sender_clone.send(command).await?;

    Ok(())
}

async fn command_receiver_loop(crawler: Crawler, mut cmd_receiver: Receiver<Command>) {
    loop {
        match cmd_receiver.recv().await {
            Some(command) => {
                info!("Received Command: {:?}", command);

                // Spawn a new task to handle the command
                let crawler_clone = crawler.clone();
                tokio::spawn(async move {
                    if let Err(command_error) = crawler_clone.handle_command(command).await {
                        print_error_and_backtrace(command_error);
                    }
                });
            }
            None => {
                CrawlerError::ReceivedNoCommandFromChannel.print();
            }
        }
    }
}