use crate::config::RssFeed;
use crate::Torrent;

use std::error::Error;

pub fn check_feed(feed: &RssFeed) -> Result<Vec<Torrent>, Box<dyn Error + Send + Sync>> {
    // Fetch the url
    let content = reqwest::blocking::get(feed.url.as_str())?.bytes()?;

    let channel = rss::Channel::read_from(&content[..])?;

    let torrents = channel
        .into_items()
        .into_iter()
        .filter_map(extract_title_and_link)
        .filter_map(|(link, title)| check_rules(feed, link, title))
        .collect();

    Ok(torrents)
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

fn check_rules(feed: &RssFeed, link: String, title: String) -> Option<Torrent> {
    for rule in &feed.rules {
        if rule.check(&title) {
            return Some(Torrent {
                link,
                title,
                download_dir: rule.download_dir.clone(),
                labels: rule.labels.clone(),
                rule: rule.filter.clone(),
            });
        }
    }
    None
}
