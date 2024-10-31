use crate::config::RssFeed;
use crate::{Torrent, TIMEOUT};

use std::error::Error;

pub struct Client {
    http_client: reqwest::Client,
    db: kv::Store<String>,
    retry_db: kv::Store<Torrent>,
}

impl Client {
    pub fn new(db: kv::Store<String>, retry_db: kv::Store<Torrent>) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            db,
            retry_db,
        }
    }

    pub async fn check_feed(&self, feed: RssFeed) -> Vec<Torrent> {
        self.fetch_torrents(&feed).await.unwrap_or(Vec::new())
    }

    async fn fetch_torrents(
        &self,
        feed: &RssFeed,
    ) -> Result<Vec<Torrent>, Box<dyn Error + Send + Sync>> {
        // Fetch the url
        let content = self
            .http_client
            .get(feed.url.as_str())
            .timeout(TIMEOUT)
            .send()
            .await?
            .bytes()
            .await?;

        let channel = rss::Channel::read_from(&content[..])?;

        let torrents = channel
            .into_items()
            .into_iter()
            .filter_map(extract_title_and_link)
            .filter(|(link, _)| !self.was_processed(link))
            .filter_map(|(link, title)| check_rules(feed, link, title))
            .collect();

        Ok(torrents)
    }

    fn was_processed(&self, link: &str) -> bool {
        is_in_db(&self.db, link) || is_in_db(&self.retry_db, link)
    }
}

fn extract_title_and_link(item: rss::Item) -> Option<(String, String)> {
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

fn is_in_db<T>(db: &kv::Store<T>, link: &str) -> bool {
    db.contains(link).unwrap_or_else(|err| {
        log::error!("Error looking for `{link}` in database: {err}");
        false
    })
}

fn check_rules(feed: &RssFeed, link: String, title: String) -> Option<Torrent> {
    for rule in &feed.rules {
        if rule.check(&title) {
            log::info!("{}:`{title}` matches rule `{}`", feed.name, rule.filter);
            return Some(Torrent {
                link,
                title,
                download_dir: rule.download_dir.clone(),
                labels: rule.labels.clone(),
            });
        }
    }
    None
}
