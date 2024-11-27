use anyhow::Error;
use thiserror::Error;
use url::ParseError;

#[derive(Error, Debug, Clone)]
pub enum CrawlerError {
    #[error("Failed to receive an error from the command channel.")]
    ReceivedNoCommandFromChannel,
    #[error("Failed to resolve relative URL: {0}")]
    FailedToResolveRelativeUrl(String),
    #[error("Link URL ({0}) does not match Base URL ({1})")]
    LinkUrlDoesNotMatchBaseUrl(String, String),
    #[error("Cannot parse link URL: {0}")]
    CannotParseLinkUrl(ParseError),
    #[error("Cannot crawl URL ({0}), because BaseURL ({1}) has stopped crawling.")]
    BaseUrlHasStoppedCrawling(String, String),
    #[error("Parent URL worker not found: {0}")]
    ParentUrlWorkerNotFound(String),
    #[error("Base URL not found: {0}")]
    BaseUrlNotFound(String),
}

impl CrawlerError {
    pub fn should_display_error(&self) -> bool {
        match self {
            CrawlerError::LinkUrlDoesNotMatchBaseUrl(_, _) | CrawlerError::BaseUrlHasStoppedCrawling(_, _) => false,
            _ => true,
        }
    }

    pub fn should_display_backtrace(&self) -> bool {
        match self {
            CrawlerError::BaseUrlHasStoppedCrawling(_, _) => false,
            _ => true,
        }
    }

    pub fn print(self) {
        if self.should_display_error() {
            if self.should_display_backtrace() {
                print_error_and_backtrace(self.into());
            } else {
                print_error(self.into());
            }
        }
    }
}

pub(crate) fn print_error(err: Error) {
    eprintln!("Error: {:#}", err);
}

pub(crate) fn print_error_and_backtrace(err: Error) {
    let backtrace = err.backtrace();
    eprintln!("Error: {:#}, Backtrace:\n{}", err, backtrace);
}