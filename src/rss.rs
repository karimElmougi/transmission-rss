use crate::config::RssFeed;

use std::error::Error;
use std::path::Path;

use rss::{Channel, Item};
use transmission_rpc::types::TorrentAddArgs;
use transmission_rpc::TransClient;

pub async fn process_feed(
    feed: &RssFeed,
    db: &kv::Store<String>,
    client: &mut TransClient,
    base_dir: &Path,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    log::info!("Processing feed `{}`", feed.title);

    // Fetch the url
    let content = reqwest::get(&feed.url).await?.bytes().await?;
    let channel = Channel::read_from(&content[..])?;

    // Filters the results
    let items = channel
        .items()
        .iter()
        .filter_map(extract)
        .filter(|(link, _)| !is_in_db(db, link));

    for (link, title) in items {
        for rule in &feed.rules {
            if title.contains(&rule.filter) {
                log::info!("`{title}` matches rule `{}`, adding torrent...", rule.filter);
                let dir = base_dir.join(&rule.download_dir);
                add_torrent(client, link, title, &dir, db).await?;
            }
        }
    }

    Ok(())
}

async fn add_torrent(
    client: &mut TransClient,
    link: &str,
    title: &String,
    download_dir: &Path,
    db: &kv::Store<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Add the torrent into transmission
    let add: TorrentAddArgs = TorrentAddArgs {
        filename: Some(link.to_string()),
        download_dir: Some(download_dir.to_string_lossy().to_string()),
        ..TorrentAddArgs::default()
    };

    let response = client.torrent_add(add).await?;
    if response.is_ok() {
        match db.set(link, title) {
            Ok(_) => log::info!("{:?} saved into db!", &link),
            Err(err) => log::error!("Failed to save {link:?} into db: {err:?}"),
        }
    } else {
        log::error!("Failed to add torrent");
    }

    Ok(())
}

fn extract(item: &Item) -> Option<(&str, &String)> {
    let link = match item.enclosure() {
        Some(enclosure) if enclosure.mime_type() == "application/x-bittorrent" => {
            Some(enclosure.url())
        }
        _ => item.link(),
    };

    match (link, &item.title) {
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
