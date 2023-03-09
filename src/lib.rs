pub mod config;
pub mod rss;

use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use tokio::time::timeout;
use transmission_rpc::types::TorrentAddArgs;
use transmission_rpc::SharableTransClient;

const TIMEOUT: Duration = Duration::from_secs(5);

pub struct Torrent {
    pub link: String,
    pub title: String,
    pub download_dir: PathBuf,
}

pub async fn add_torrent(client: &SharableTransClient, torrent: Torrent, db: &kv::Store<String>) {
    match timeout(TIMEOUT, add_torrent_impl(client, &torrent, db)).await {
        Ok(Ok(())) => (),
        Ok(Err(err)) => log::error!("Error while adding torrent `{}`: {err}", torrent.title),
        Err(_) => log::error!("Timeout while connecting to Transmission"),
    }
}

async fn add_torrent_impl(
    client: &SharableTransClient,
    torrent: &Torrent,
    db: &kv::Store<String>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let add: TorrentAddArgs = TorrentAddArgs {
        filename: Some(torrent.link.to_string()),
        download_dir: Some(torrent.download_dir.to_string_lossy().to_string()),
        ..TorrentAddArgs::default()
    };

    let response = client.torrent_add(add).await?;
    if response.is_ok() {
        if let Err(err) = async { db.set(&torrent.link, &torrent.title) }.await {
            log::error!("Failed to save {:?} into db: {err:?}", torrent.link);
        }
    } else {
        log::error!("Failed to add torrent");
    }

    Ok(())
}
