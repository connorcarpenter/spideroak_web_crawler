use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Start(String), // Start crawling the provided URL
    Stop(String),  // Stop crawling the provided URL
    List,          // List all the crawled URLs
}
