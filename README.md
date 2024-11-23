# spideroak_web_crawler

### Technical Test Developer Platform Team

The following test can be implemented in any programming language.
You can take as much time as you need, but it's not expected that you spend more than 2 hours on it.

The test consists of implementing a "Web Crawler as a Service."
The application consists of a command line client and a local service (daemon) which performs the actual web crawling.
The communication between client and server should use a form of IPC mechanism.
For each URL, the Web Crawler creates a tree of links with the root of the tree being the root URL.
The crawler should only follow links on the domain of the provided URL and not follow external links.
Bonus points for making it as fast as possible.

The command line client should provide the following operations:

```
$ crawl -start www.example.com # signals the service to start crawling www.example.com
$ crawl -stop www.example.com  # signals the service to stop crawling www.example.com
$ crawl -list                  # shows the current "site tree" for all crawled URLs.
```

Notes:
- You can use external packages/libraries.