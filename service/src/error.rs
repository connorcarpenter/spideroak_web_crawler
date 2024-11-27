use anyhow::Error;
use thiserror::Error;
use url::ParseError;

#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("Failed to receive an error from the command channel.")]
    ReceivedNoCommandFromChannel,
    #[error("Failed to resolve relative URL: {0}")]
    FailedToResolveRelativeUrl(String),
    #[error("Link URL ({0}) does not match Base URL ({1})")]
    LinkUrlDoesNotMatchBaseUrl(String, String),
    #[error("Cannot parse link URL: {0}")]
    CannotParseLinkUrl(ParseError),
}

impl CrawlerError {
    pub fn should_display_error(&self) -> bool {
        match self {
            CrawlerError::ReceivedNoCommandFromChannel => true,
            CrawlerError::FailedToResolveRelativeUrl(_) => true,
            CrawlerError::LinkUrlDoesNotMatchBaseUrl(_, _) => false,
            CrawlerError::CannotParseLinkUrl(_) => true,
        }
    }

    pub fn print(self) {
        if self.should_display_error() {
            print_error(self.into());
        }
    }
}

pub(crate) fn print_error(err: Error) {
    let backtrace = err.backtrace();
    eprintln!("Error: {:#}, Backtrace:\n{}", err, backtrace);
}