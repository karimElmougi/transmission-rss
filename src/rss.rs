use crate::config::RssFeed;

use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rss::{Channel, Item};
use transmission_rpc::types::TorrentAddArgs;
use transmission_rpc::TransClient;

pub struct Torrent {
    pub link: String,
    pub title: String,
    pub download_dir: PathBuf,
}

pub async fn check_feed(
    feed: RssFeed,
    db: &kv::Store<String>,
    download_dir: &Path,
    client: &mut TransClient,
) {
    const TIMEOUT: Duration = Duration::from_secs(5);
    let torrents =
        match tokio::time::timeout(TIMEOUT, fetch_torrents(&feed, db, download_dir)).await {
            Ok(Ok(torrents)) => torrents,
            Ok(Err(err)) => {
                log::error!("Couldn't fetch torrent for feed `{}`: {err}", feed.name);
                return;
            }
            Err(_) => {
                log::error!("Connection timeout while fetching feed `{}`", feed.name);
                return;
            }
        };

    for torrent in torrents {
        match tokio::time::timeout(TIMEOUT, add_torrent(client, &torrent, db)).await {
            Ok(Ok(())) => (),
            Ok(Err(err)) => log::error!("Error while adding torrent `{}`: {err}", torrent.title),
            Err(_) => log::error!("Timeout while connecting to Transmission"),
        }
    }
}

async fn add_torrent(
    client: &mut TransClient,
    torrent: &Torrent,
    db: &kv::Store<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Add the torrent into transmission
    let add: TorrentAddArgs = TorrentAddArgs {
        filename: Some(torrent.link.to_string()),
        download_dir: Some(torrent.download_dir.to_string_lossy().to_string()),
        ..TorrentAddArgs::default()
    };

    let response = client.torrent_add(add).await?;
    if response.is_ok() {
        match db.set(&torrent.link, &torrent.title) {
            Ok(_) => log::info!("{:?} saved into db!", torrent.link),
            Err(err) => log::error!("Failed to save {:?} into db: {err:?}", torrent.link),
        }
    } else {
        log::error!("Failed to add torrent");
    }

    Ok(())
}

async fn fetch_torrents(
    feed: &RssFeed,
    db: &kv::Store<String>,
    base_dir: &Path,
) -> Result<Vec<Torrent>, Box<dyn Error + Send + Sync>> {
    log::info!("Checking feed `{}`", feed.name);

    // Fetch the url
    let content = reqwest::get(feed.url.as_str()).await?.bytes().await?;
    let channel = Channel::read_from(&content[..])?;

    // Filters the results
    let torrents = async {
        channel
            .into_items()
            .into_iter()
            .filter_map(extract_title_and_link)
            .filter(|(link, _)| !is_in_db(db, link))
            .filter_map(|(link, title)| {
                for rule in &feed.rules {
                    if rule.check(&title) {
                        log::info!("`{title}` matches rule `{}`", rule.filter);
                        let dir = base_dir.join(&rule.download_dir);
                        return Some(Torrent {
                            link,
                            title,
                            download_dir: dir,
                        });
                    }
                }
                None
            })
            .collect()
    }
    .await;

    Ok(torrents)
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
