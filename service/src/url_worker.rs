use anyhow::Result;
use chrono::{DateTime, Local};
use log::info;
use url::{ParseError, Url};

use crate::{
    crawler::Crawler,
    error::{print_error_and_backtrace, CrawlerError},
    parser::find_anchors,
};

const URL_MAX_STALE_MINUTES: i64 = 1;
const PARSER_WORKER_COUNT: usize = 4;

pub struct UrlWorker {
    crawler: Crawler,
    url: Url,
    last_access_timestamp: Option<DateTime<Local>>,
}

impl UrlWorker {
    pub fn new(crawler: Crawler, url: &Url) -> Result<Self> {
        let url = url.clone();

        Ok(Self {
            crawler,
            url,
            last_access_timestamp: None,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        if let Some(timestamp) = self.last_access_timestamp {
            let now = Local::now();
            let duration = now.signed_duration_since(timestamp);
            if duration.num_minutes() < URL_MAX_STALE_MINUTES {
                // info!("Skip Crawling URL (already fetched within last 5 minutes): {}", self.url);
                return Ok(());
            }
        }

        // Fetch page content using reqwest
        info!("Crawling URL: {}", self.url);
        let document = reqwest::get(self.url.clone()).await?.text().await?;
        // info!("Received response from URL: {}", self.url);

        // store timestamp
        self.last_access_timestamp = Some(Local::now());

        // Spin up Parser Workers
        for worker_index in 0..PARSER_WORKER_COUNT {
            let crawler_clone = self.crawler.clone();
            let url_clone = self.url.clone();
            let document_clone = document.clone();
            tokio::spawn(async move {
                Self::parser_worker(crawler_clone, worker_index, url_clone, document_clone);
            });
        }

        Ok(())
    }

    fn parser_worker(crawler: Crawler, worker_index: usize, previous_url: Url, document: String) {
        for link_url in find_anchors(document.as_str(), worker_index, PARSER_WORKER_COUNT) {
            match Self::parser_worker_handle_link(&previous_url, link_url.as_str()) {
                Ok(link_url) => {
                    let previous_url_clone = previous_url.clone();
                    let crawler = crawler.clone();
                    tokio::spawn(async move {
                        if let Err(err) = crawler
                            .start_job(Some(&previous_url_clone), &link_url)
                            .await
                        {
                            print_error_and_backtrace(err);
                        }
                    });
                }
                Err(err) => {
                    err.print();
                }
            }
        }
    }

    fn parser_worker_handle_link(previous_url: &Url, link_url: &str) -> Result<Url, CrawlerError> {
        let link_url = match Url::parse(link_url) {
            Ok(url) => url,
            Err(
                ParseError::RelativeUrlWithoutBase
                | ParseError::RelativeUrlWithCannotBeABaseBase
                | ParseError::EmptyHost
                | ParseError::SetHostOnCannotBeABaseUrl,
            ) => {
                if let Ok(resolved_url) = previous_url.join(link_url) {
                    resolved_url
                } else {
                    return Err(CrawlerError::FailedToResolveRelativeUrl(
                        link_url.to_string(),
                    ));
                }
            }
            Err(err) => {
                return Err(CrawlerError::CannotParseLinkUrl(err));
            }
        };
        if !have_same_base(previous_url, &link_url) {
            let err = CrawlerError::LinkUrlDoesNotMatchBaseUrl(
                previous_url.to_string(),
                link_url.to_string(),
            )
            .into();
            return Err(err);
        }
        Ok(link_url)
    }
}

fn have_same_base(url1: &Url, url2: &Url) -> bool {
    url1.scheme() == url2.scheme() && url1.host_str() == url2.host_str()
}
