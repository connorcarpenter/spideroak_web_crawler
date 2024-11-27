pub(crate) struct BaseUrl {
    crawling: bool,
}

impl BaseUrl {
    pub(crate) fn new() -> Self {
        Self { crawling: false }
    }

    pub(crate) fn start_crawling(&mut self) {
        self.crawling = true;
    }

    pub(crate) fn stop_crawling(&mut self) {
        self.crawling = false;
    }

    pub(crate) fn is_crawling(&self) -> bool {
        self.crawling
    }
}
