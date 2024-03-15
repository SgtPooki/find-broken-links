# Webcrawler in Rust

## Project Description

This project is a simple webcrawler written in Rust. It starts from a given domain and recursively crawls through the pages, collecting URLs that result in a 404 status code. The primary goal of this crawler is to help identify broken links within a website.

## Getting Started

To run this project, you will need Rust installed on your machine. You can install Rust through `rustup` by following the instructions on the [official Rust website](https://www.rust-lang.org/learn/get-started).

Once Rust is installed, clone this repository and navigate into the project directory:

```bash
git clone <repository-url>
cd webcrawler
```

To run the crawler:

```bash
cargo run http://example.com
```

Filter based on domain name:

```bash
cargo run http://example.com example.com
```

## Roadmap

As this project is in its early stages, there are several potential enhancements and features to consider for future development:

- **Concurrency**: Introduce parallel HTTP requests to improve the efficiency and speed of the crawling process, allowing multiple pages to be crawled simultaneously.

- **Robust Error Handling and Retries**: Enhance error handling mechanisms to manage failed requests more gracefully, including implementing retries for temporary issues.

- **Result Storage**: Develop a system for storing crawl results, potentially in a file or database, for further analysis and reporting.

- **Configurable Depth Limit**: Add an option to limit the crawl depth to prevent the crawler from going too deep into a website, which can be useful for large sites.

- **Rate Limiting**: Implement rate limiting to control the crawler's request rate, ensuring that it does not overwhelm the target servers.

- **User-Agent Customization**: Allow customization of the `User-Agent` string in HTTP requests to identify the crawler politely and comply with website policies.

- **Logging and Monitoring**: Improve logging for debugging and monitoring purposes, providing insights into the crawler's operation and performance.

## TODO:

- [ ] make crawling iterative instead of recursive
- [ ] add tests
- [ ] pull out web crawling logic into separate module

## Contributing

Contributions to this project are welcome! Please feel free to submit issues or pull requests with improvements or new features.

## License

This project is licensed under the [MIT License](LICENSE). See the LICENSE file for more details.
