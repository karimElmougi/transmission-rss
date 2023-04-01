use crate::config::RssFeed;
use crate::{Torrent, TIMEOUT};

use std::error::Error;
use std::path::{Path, PathBuf};

use rss::{Channel, Item};
use tokio::time::timeout;

pub struct Client {
    http_client: reqwest::Client,
    db: kv::Store<String>,
    base_download_dir: PathBuf,
}

impl Client {
    pub fn new(db: kv::Store<String>, base_download_dir: PathBuf) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            db,
            base_download_dir,
        }
    }

    pub async fn check_feed(&self, feed: RssFeed) -> Vec<Torrent> {
        match timeout(TIMEOUT, self.fetch_torrents(&feed)).await {
            Ok(Ok(torrents)) => torrents,
            Ok(Err(err)) => {
                log::error!("Couldn't fetch torrent for feed `{}`: {err}", feed.name);
                vec![]
            }
            Err(_) => {
                log::error!("Connection timeout while fetching feed `{}`", feed.name);
                vec![]
            }
        }
    }

    async fn fetch_torrents(
        &self,
        feed: &RssFeed,
    ) -> Result<Vec<Torrent>, Box<dyn Error + Send + Sync>> {
        // Fetch the url
        let content = self
            .http_client
            .get(feed.url.as_str())
            .send()
            .await?
            .bytes()
            .await?;

        let channel = Channel::read_from(&content[..])?;

        let torrents = async {
            channel
                .into_items()
                .into_iter()
                .filter_map(extract_title_and_link)
                .filter(|(link, _)| !is_in_db(&self.db, link))
                .filter_map(|(link, title)| check_rules(feed, &self.base_download_dir, link, title))
                .collect()
        }
        .await;

        Ok(torrents)
    }
}

fn extract_title_and_link(item: Item) -> Option<(String, String)> {
    let link = match item.enclosure {
        Some(enclosure) if enclosure.mime_type() == "application/x-bittorrent" => {
            Some(enclosure.url)
        }
        _ => item.link,
    };

    match (link, item.title) {
        (Some(link), Some(title)) => Some((link, title)),
        (None, Some(title)) => {
            log::warn!("No link for `{title}`");
            None
        }
        (Some(link), None) => {
            log::warn!("No title for `{link}`");
            None
        }
        _ => None,
    }
}

fn is_in_db(db: &kv::Store<String>, link: &str) -> bool {
    match db.get(link) {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(err) => {
            log::error!("Error looking for `{link}` in database: {err}");
            false
        }
    }
}

fn check_rules(feed: &RssFeed, base_dir: &Path, link: String, title: String) -> Option<Torrent> {
    for rule in &feed.rules {
        if rule.check(&title) {
            log::info!("{}:`{title}` matches rule `{}`", feed.name, rule.filter);
            return Some(Torrent {
                link,
                title,
                download_dir: base_dir.join(&rule.download_dir),
                labels: rule.labels.clone(),
            });
        }
    }
    None
}
