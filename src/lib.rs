pub mod config;
pub mod rss;

use std::path::PathBuf;
use std::time::Duration;

use tokio::time::timeout;
use transmission_rpc::types::TorrentAddArgs;
use transmission_rpc::SharableTransClient;

const TIMEOUT: Duration = Duration::from_secs(5);

pub struct Torrent {
    pub link: String,
    pub title: String,
    pub download_dir: PathBuf,
    pub labels: Vec<String>,
}

pub async fn add_torrent(client: &SharableTransClient, torrent: Torrent, db: &kv::Store<String>) {
    if let Err(_) = timeout(TIMEOUT, add_torrent_impl(client, torrent, db)).await {
        log::error!("Timeout while connecting to Transmission");
    }
}

async fn add_torrent_impl(client: &SharableTransClient, torrent: Torrent, db: &kv::Store<String>) {
    let add: TorrentAddArgs = TorrentAddArgs {
        filename: Some(torrent.link.to_string()),
        download_dir: Some(torrent.download_dir.to_string_lossy().to_string()),
        labels: Some(torrent.labels),
        ..TorrentAddArgs::default()
    };

    let response = match client.torrent_add(add).await {
        Ok(response) => response,
        Err(err) => {
            log::error!("Error connecting to Transmission: {err}");
            return;
        }
    };

    if response.is_ok() {
        if let Err(err) = async { db.set(&torrent.link, &torrent.title) }.await {
            log::error!(
                "Failed to save link for `{}` into db: {err:?}",
                torrent.title
            );
        }
    } else {
        let reason = response.result;
        log::error!("Failed to add torrent `{}`: `{reason}`", torrent.title);
    }
}
