use serde::{Deserialize, Serialize};
use serde_json;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tokio::signal;
use url::{ParseError, Url};
mod debug_channel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    // Collect command line arguments
    let args: Vec<String> = env::args().collect();

    // Expect at least one argument for the domain
    if args.len() < 2 {
        return Err("Usage: find-broken-links <domain> [<fuzzy_match_string>]".into());
    }

    let root_url = args[1].clone();
    // This is the fuzzy match string
    let fuzzy_match_string = match args.get(2).cloned() {
        Some(s) => Some(s),
        None => None,
    };

    // Validate the URL format
    let parsed_url = Url::parse(&root_url).expect("Invalid URL format provided");

    // Extract the hostname from the parsed URL
    let hostname = parsed_url.host_str().ok_or("Invalid hostname")?.to_string();

    log::info!("Starting to crawl: {}", parsed_url.to_string());

    // send/receive channels for urls that are found to be emitted.
    let mut debug_channel = debug_channel::DebugChannel::<Option<String>>::new(5);

    // Clone the sender to move into the async block
    let debug_sender = debug_channel.sender(); // This is a DebugSender with tracking
                                               // Spawn the crawler task
    tokio::spawn(async move {
        if let Err(e) = crawl_and_collect_404s(parsed_url, debug_sender, fuzzy_match_string).await {
            log::error!("Crawler error: {}", e);
        }
        log::info!("tokio::spawn block is done...")
    });

    // Wait for either CTRL+C or the crawler task to finish
    let mut not_found_urls = Vec::new();
    loop {
        log::debug!("Waiting for messages or completion signal...");
        tokio::select! {
            message = debug_channel.recv() => {
                match message {
                    Some(Some(url)) => {
                        not_found_urls.push(url);
                    },
                Some(None) => { // Completion signal received
                    log::info!("Crawl complete, ending loop.");
                    break;
                },
                    None => {
                        log::info!("Channel closed, ending loop.");
                        break; // Break the loop when the channel is closed
                    }
                }
            },
            _ = signal::ctrl_c() => {
                log::info!("Received CTRL+C, shutting down...");
                break; // Also break the loop on CTRL+C
            },
        }
    }

    // Convert not found URLs into NotFoundError structs
    let not_found_errors: Vec<NotFoundError> = not_found_urls
        .into_iter()
        .map(|url| NotFoundError {
            url,
            title: None, // You would extract the title in your actual crawling logic
        })
        .collect();

    // Construct the file path
    let file_name = format!("./results/{}.json", hostname);
    let file_path = Path::new(&file_name);

    if not_found_errors.len() > 0 {
        log::info!("Saving {} 404 urls...", not_found_errors.len());
        // Save the not found errors
        save_not_found_errors(&not_found_errors, file_path)?;
    } else {
        log::info!("No 404s found")
    }

    // log how big the mpsc channel buffer got so we can change if needed:

    log::debug!(
        "mpsc channel got to max size of {}",
        debug_channel.get_max_buffer_size()
    );

    Ok(())
}

fn make_absolute_url(base_url: &Url, link: &str) -> Result<Url, ParseError> {
    // let base = Url::parse(base_url)?;
    base_url.join(link) // This resolves the relative URL 'link' against the base URL 'base_url'
}

async fn fetch_html(url: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/58.0.3029.110 Safari/537.3")
        .build()?;

    let resp = client.get(url).send().await?;

    if resp.status().is_success() {
        resp.text().await
    } else {
        // Directly return the error without constructing a new one
        Err(resp.error_for_status().unwrap_err())
    }
}

// TODO: return the html element along with the link
fn find_links(html: &str) -> Vec<String> {
    let document = select::document::Document::from(html);
    let mut links = Vec::new();
    let denied_protocols = ["mailto:", "ftp:", "tel:"];
    let denied_links = ["#", "javascript:void(0)"];

    for node in document.find(select::predicate::Name("a")) {
        if let Some(link) = node.attr("href") {
            if !denied_protocols
                .iter()
                .any(|&protocol| link.starts_with(protocol))
            {
                if !denied_links.iter().any(|&denied| link == denied){
                    log::debug!("Adding link: {}", link.to_string());
                    links.push(link.to_string());
                }
            }
        }
    }

    links
}

async fn crawl_and_collect_404s(
    root_url: Url,
    tx: debug_channel::DebugSender<Option<String>>,
    fuzzy_match_string: Option<String>,
) -> Result<(), anyhow::Error> {
    log::info!("crawling and collecting 404s");
    let root_domain = root_url
        .domain()
        .ok_or_else(|| anyhow::anyhow!("Root URL has no domain"))?;
    let mut to_visit = vec![root_url.to_string()];
    let mut visited = Vec::new();

    while let Some(url) = to_visit.pop() {
        if visited.contains(&url) {
            continue;
        }
        log::info!("crawling {}", url);

        let html_result = fetch_html(&url).await;
        match html_result {
            Ok(html) => {
                // TODO: save the url
                let links = find_links(&html);
                for link in links {
                    let absolute_link = make_absolute_url(&root_url, &link)?;
                    let absolute_link_domain = match absolute_link.domain() {
                        Some(domain) => domain,
                        None => {
                            log::warn!("Link '{}' has no domain, skipping...", absolute_link);
                            continue;
                        }
                    };
                    let matches_exact = root_domain == absolute_link_domain;
                    let matches_fuzzy = fuzzy_match_string
                        .as_ref()
                        .map(|fuzzy_match_string| {
                            absolute_link_domain
                                .to_string()
                                .contains(fuzzy_match_string)
                        })
                        .unwrap_or(false);
                    if (matches_exact || matches_fuzzy)
                        && !visited.contains(&absolute_link.to_string())
                    {
                        to_visit.push(absolute_link.to_string());
                    }
                }
            }
            Err(e) if e.status() == Some(reqwest::StatusCode::NOT_FOUND) => {
                if let Err(send_err) = tx.send(Some(url.clone())).await {
                    log::error!("Failed to send 404 URL through the channel: {}", send_err);
                }
            }
            Err(e) => return Err(e.into()),
        }

        visited.push(url);
    }
    log::info!("Done crawling...");
    if let Err(send_err) = tx.send(None).await {
        log::error!(
            "Failed to signal completion through the channel: {}",
            send_err
        );
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct NotFoundError {
    url: String,
    title: Option<String>, // Titles can be optional since some 404 pages might not have a clear title
}

fn save_not_found_errors(errors: &[NotFoundError], file_path: &Path) -> std::io::Result<()> {
    fs::create_dir_all(file_path.parent().unwrap())?; // Ensure the directory exists

    let mut file = File::create(file_path)?;
    let data = serde_json::to_string_pretty(&errors)?;
    file.write_all(data.as_bytes())?;

    Ok(())
}
