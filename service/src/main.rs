use std::{sync::{Arc, Mutex}, collections::{HashMap, HashSet}};

use bincode;
use scraper::{Html, Selector};
use log::{info, error, warn};
use tokio::{sync::mpsc, io::AsyncReadExt, net::TcpListener};

use shared::Command;

#[derive(Debug, Clone)]
struct CrawlJob {
    url: String,
    in_progress: bool,
    children: Vec<String>,
}

// Shared State Type
type SharedState = Arc<Mutex<HashMap<String, CrawlJob>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting Web Crawler Daemon on 127.0.0.1:8080");

    // Shared State
    let shared_state: SharedState = Arc::new(Mutex::new(HashMap::new()));

    // Channel to receive commands from client (e.g., start, stop, list)
    let (cmd_sender, mut cmd_receiver) = mpsc::channel::<Command>(32);

    // Launch command handler task
    let state_clone = shared_state.clone();
    tokio::spawn(async move {
        loop {
            if let Some(command) = cmd_receiver.recv().await {
                info!("Command receiver channel received: {:?}", command);
                handle_command(command, state_clone.clone()).await;
            } else {
                warn!("Command receiver channel received None");
                continue;
            }
        }
    });

    // Start TCP listener for IPC
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    loop {
        let (mut socket, addr) = listener.accept().await?;
        let sender_clone = cmd_sender.clone();
        tokio::spawn(async move {
            let mut buf = [0; 1024];
            match socket.read(&mut buf).await {
                Ok(n) => {
                    info!("Received TCP message from: {:?}", addr);
                    // Deserialize command from received bytes using bincode
                    if let Ok(command) = bincode::deserialize::<Command>(&buf[..n]) {
                        info!("Sending to command channel: {:?}", command);
                        sender_clone.send(command).await.unwrap(); // Send command to the command handler
                    } else {
                        warn!("Failed to deserialize command");
                    }
                }
                Err(e) => {
                    error!("Failed to read from socket: {:?}", e);
                }
            }
        });
    }
}

async fn handle_command(command: Command, state: SharedState) {
    match command {
        Command::Start(url) => {
            info!("Starting crawl for URL: {}", url);
            let state_clone = state.clone();
            // Launch a new task to crawl the website
            tokio::spawn(async move {
                if let Err(e) = crawl_website(url, state_clone).await {
                    error!("Error during crawling: {:?}", e);
                }
            });
        }
        Command::Stop(url) => {
            info!("Stopping crawl for URL: {}", url);
            let mut state = state.lock().unwrap();
            // Set the in_progress flag to false to signal the job should stop
            if let Some(job) = state.get_mut(&url) {
                job.in_progress = false;
            }
        }
        Command::List => {
            info!("Listing all crawled sites...");
            let state = state.lock().unwrap();
            // Iterate over all crawled URLs and print their details
            for (url, job) in state.iter() {
                println!("URL: {}, Children: {:?}", url, job.children);
            }
        }
    }
}

async fn crawl_website(url: String, state: SharedState) -> Result<(), Box<dyn std::error::Error>> {
    let mut visited = HashSet::new(); // Track visited URLs to prevent re-crawling
    let mut to_visit = vec![url.clone()]; // URLs to be visited, starting with the root URL

    while let Some(current_url) = to_visit.pop() {
        if visited.contains(&current_url) {
            continue; // Skip URLs that have already been visited
        }

        info!("Crawling URL: {}", current_url);
        visited.insert(current_url.clone()); // Mark the URL as visited

        // Fetch page content using reqwest
        let body = reqwest::get(&current_url).await?.text().await?;
        let document = Html::parse_document(&body); // Parse the HTML document
        let selector = Selector::parse("a").unwrap(); // Create a selector to find all anchor tags

        let mut children = Vec::new();
        for element in document.select(&selector) {
            if let Some(link) = element.value().attr("href") {
                let link = link.to_string();
                if is_same_domain(&url, &link) {
                    children.push(link.clone()); // Add link to children if it's in the same domain
                    to_visit.push(link); // Add link to visit queue
                }
            }
        }

        // Update shared state with the current crawl job
        let mut state = state.lock().unwrap();
        state.insert(
            current_url.clone(),
            CrawlJob {
                url: current_url,
                in_progress: true,
                children,
            },
        );
    }

    Ok(())
}

fn is_same_domain(root: &str, link: &str) -> bool {
    // A simple domain check (for illustration purposes)
    link.contains(root) // Check if the link contains the root URL
}