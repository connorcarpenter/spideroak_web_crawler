use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use log::{info};
use tokio::sync::RwLock;
use url::Url;

use shared::Command;

use crate::url_worker::UrlWorker;

#[derive(Clone)]
pub struct Crawler {
    url_workers: Arc<RwLock<HashMap<Url, Arc<RwLock<UrlWorker>>>>>,
}

impl Crawler {
    pub fn new() -> Self {
        Self {
            url_workers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn has_job(&self, url: &Url) -> bool {
        let map = self.url_workers.read().await;
        map.contains_key(url)
    }

    async fn create_job(&self, url: &Url) -> Result<()> {
        let self_clone = self.clone();
        let mut map = self.url_workers.write().await;
        let crawl_job = UrlWorker::new(self_clone, url)?;
        map.insert(url.clone(), Arc::new(RwLock::new(crawl_job)));
        Ok(())
    }

    async fn get_job(&self, url: &Url) -> Option<Arc<RwLock<UrlWorker>>> {
        let map = self.url_workers.read().await;
        map.get(url).cloned()
    }

    pub async fn handle_command(&self, command: Command) -> Result<()> {
        match command {
            Command::Start(url) => self.handle_command_start(&url).await,
            Command::Stop(url) => self.handle_command_stop(&url).await,
            Command::List => self.handle_command_list().await,
        }
    }

    async fn handle_command_start(&self, url_str: &str) -> Result<()> {

        let url = Url::parse(url_str)?;

        self.start_job(&url).await?;

        Ok(())
    }

    pub(crate) async fn start_job(&self, url: &Url) -> Result<()> {

        // sanitize url
        let mut url = url.clone();

        // get rid of querystring
        url.set_query(None);

        // get rid of trailing slash
        if url.path().ends_with('/') {
            let mut path = url.path().to_owned();
            path.pop();  // Remove the trailing slash
            url.set_path(&path);
        }

        // check if job already exists
        if !self.has_job(&url).await {
            self.create_job(&url).await?;
        }

        // start the job
        let job = self.get_job(&url).await.unwrap();
        job.write().await.start().await?;

        Ok(())
    }

    async fn handle_command_stop(&self, url: &str) -> Result<()> {
        info!("Stopping crawl for URL: {}", url);

        todo!();

        Ok(())
    }

    async fn handle_command_list(&self) -> Result<()> {
        info!("Listing all crawled sites...");

        todo!();

        Ok(())
    }
}