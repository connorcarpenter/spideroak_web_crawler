use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Result;
use log::{info, warn};
use tokio::sync::RwLock;
use url::Url;

use shared::Command;

use crate::{
    base_url::BaseUrl,
    error::{print_error, CrawlerError},
    url_worker::UrlWorker,
};

#[derive(Clone)]
pub struct Crawler {
    base_urls: Arc<RwLock<HashMap<Url, BaseUrl>>>,
    url_workers: Arc<RwLock<HashMap<Url, Arc<RwLock<UrlWorker>>>>>,
    url_parents: Arc<RwLock<HashMap<Url, HashSet<Url>>>>,
}

impl Crawler {
    pub fn new() -> Self {
        Self {
            base_urls: Arc::new(RwLock::new(HashMap::new())),
            url_workers: Arc::new(RwLock::new(HashMap::new())),
            url_parents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn has_worker(&self, url: &Url) -> bool {
        let map = self.url_workers.read().await;
        map.contains_key(url)
    }

    async fn create_worker(&self, prev_url_opt: Option<&Url>, url: &Url) -> Result<()> {
        let self_clone = self.clone();

        // Add to children
        if let Some(prev_url) = prev_url_opt {
            // Parent is UrlWorker
            let mut map = self.url_parents.write().await;
            if let Some(children) = map.get_mut(prev_url) {
                children.insert(url.clone());
            } else {
                return Err(CrawlerError::ParentUrlWorkerNotFound(prev_url.to_string()).into());
            }
        } else {
            // Parent is BaseUrl
            let base_url = strip_url_to_domain(url.clone());
            if url.as_str() != base_url.as_str() {
                let mut map = self.url_parents.write().await;
                if let Some(children) = map.get_mut(&base_url) {
                    children.insert(url.clone());
                } else {
                    return Err(CrawlerError::BaseUrlNotFound(url.to_string()).into());
                }
            }
        }

        // Create new worker
        let mut map = self.url_workers.write().await;
        let crawl_job = UrlWorker::new(self_clone, url)?;
        map.insert(url.clone(), Arc::new(RwLock::new(crawl_job)));

        // Register the parent
        let mut map = self.url_parents.write().await;
        if !map.contains_key(url) {
            map.insert(url.clone(), HashSet::new());
        }

        Ok(())
    }

    async fn get_worker(&self, url: &Url) -> Option<Arc<RwLock<UrlWorker>>> {
        let map = self.url_workers.read().await;
        map.get(url).cloned()
    }

    async fn base_url_is_crawling(&self, url: &Url) -> bool {
        let base_url = strip_url_to_domain(url.clone());
        let map = self.base_urls.read().await;
        if let Some(base_url_record) = map.get(&base_url) {
            base_url_record.is_crawling()
        } else {
            false
        }
    }

    async fn base_url_start_crawling(&self, url: &Url) {
        let base_url = strip_url_to_domain(url.clone());
        let mut map = self.base_urls.write().await;
        if !map.contains_key(&base_url) {
            map.insert(base_url.clone(), BaseUrl::new());
        }
        map.get_mut(&base_url).unwrap().start_crawling();

        // Register the parent
        let mut map = self.url_parents.write().await;
        if !map.contains_key(&base_url) {
            map.insert(base_url.clone(), HashSet::new());
        }
    }

    async fn base_url_stop_crawling(&self, url: &Url) {
        let base_url = strip_url_to_domain(url.clone());
        let mut map = self.base_urls.write().await;
        if !map.contains_key(&base_url) {
            warn!("Url not found: {}", base_url);
            return;
        }
        map.get_mut(&base_url).unwrap().stop_crawling();
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

        self.base_url_start_crawling(&url).await;

        // start crawling
        self.start_job(None, &url).await?;

        Ok(())
    }

    pub(crate) async fn start_job(&self, prev_url_opt: Option<&Url>, url: &Url) -> Result<()> {
        let url = strip_url_to_domain_and_path(url.clone());

        if !self.base_url_is_crawling(&url).await {
            let crawler_error = CrawlerError::BaseUrlHasStoppedCrawling(
                url.path().to_string(),
                strip_url_to_domain(url.clone()).to_string(),
            );
            print_error(crawler_error.clone().into());
            return Ok(());
        }

        // check if job already exists
        if !self.has_worker(&url).await {
            self.create_worker(prev_url_opt, &url).await?;
        }

        // start the job
        let job = self.get_worker(&url).await.unwrap();
        job.write().await.start().await?;

        Ok(())
    }

    async fn handle_command_stop(&self, url_str: &str) -> Result<()> {
        let url = Url::parse(url_str)?;
        let url = strip_url_to_domain(url);

        self.base_url_stop_crawling(&url).await;

        info!("Stopping crawling for {}", url);

        Ok(())
    }

    async fn handle_command_list(&self) -> Result<()> {
        let mut url_set = HashSet::new();

        // list through all BaseUrls
        let base_urls = self.base_urls.read().await;
        for (url, _) in base_urls.iter() {
            url_set.insert(url.clone());
        }

        let url_parents = self.url_parents.read().await;
        let mut indentation = String::new();

        print_children(&url_parents, &mut indentation, &url_set);

        Ok(())
    }
}

fn strip_url_to_domain(mut url: Url) -> Url {
    url.set_path("");
    url.set_query(None);
    url.set_fragment(None);
    url
}

fn strip_url_to_domain_and_path(mut url: Url) -> Url {
    url.set_query(None);
    url.set_fragment(None);

    // get rid of trailing slash
    if url.path().ends_with('/') {
        let mut path = url.path().to_owned();
        path.pop(); // Remove the trailing slash
        url.set_path(&path);
    }
    url
}

fn print_children(
    url_parents: &HashMap<Url, HashSet<Url>>,
    indentation: &mut String,
    children: &HashSet<Url>,
) {
    let mut childed_urls = Vec::new();
    let mut childless_urls = Vec::new();

    for url in children.iter() {
        if let Some(children) = url_parents.get(url) {
            if children.is_empty() {
                childless_urls.push(url);
            } else {
                childed_urls.push((url, children));
            }
        } else {
            panic!("UrlWorker not found for: {}", url);
        }
    }

    for (url, children) in childed_urls {
        let url_str = if indentation.is_empty() {
            // display full url if these are base urls
            url.as_str()
        } else {
            // display only paths if the base url is known
            url.path()
        };

        info!("{}{}", indentation, url_str);
        indentation.push(' ');
        print_children(url_parents, indentation, children);
        indentation.pop();
    }

    let childless_urls: Vec<&str> = if indentation.is_empty() {
        // display full url if these are childless base urls
        childless_urls.iter().map(|url| url.as_str()).collect()
    } else {
        // display only paths if the base url is known
        childless_urls.iter().map(|url| url.path()).collect()
    };
    let childless_urls = childless_urls.join(" ");
    info!("{}{}", indentation, childless_urls);
}
