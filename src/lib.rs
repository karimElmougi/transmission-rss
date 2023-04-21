pub mod config;
pub mod rss;

use std::path::PathBuf;
use std::time::Duration;

use config::Config;
use tokio::time::timeout;
use transmission_rpc::types::{BasicAuth, TorrentAddArgs};
use transmission_rpc::SharableTransClient;

const TIMEOUT: Duration = Duration::from_secs(5);

pub struct Torrent {
    pub link: String,
    pub title: String,
    pub download_dir: PathBuf,
    pub labels: Vec<String>,
}

pub struct TransmissionClient {
    inner: SharableTransClient,
    db: kv::Store<String>,
}

impl TransmissionClient {
    pub fn new(cfg: &Config, db: kv::Store<String>) -> Self {
        let basic_auth = BasicAuth {
            user: cfg.transmission.username.clone(),
            password: cfg.transmission.password.clone(),
        };
        let inner = SharableTransClient::with_auth(cfg.transmission.url.clone(), basic_auth);

        Self { inner, db }
    }

    pub async fn add(&self, torrent: Torrent) {
        if let Err(_) = timeout(TIMEOUT, self.add_impl(torrent)).await {
            log::error!("Timeout while connecting to Transmission");
        }
    }

    async fn add_impl(&self, torrent: Torrent) {
        let add: TorrentAddArgs = TorrentAddArgs {
            filename: Some(torrent.link.to_string()),
            download_dir: Some(torrent.download_dir.to_string_lossy().to_string()),
            labels: Some(torrent.labels),
            ..TorrentAddArgs::default()
        };

        let response = match self.inner.torrent_add(add).await {
            Ok(response) => response,
            Err(err) => {
                log::error!("Error connecting to Transmission: {err}");
                return;
            }
        };

        if response.is_ok() {
            if let Err(err) = async { self.db.set(&torrent.link, &torrent.title) }.await {
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
}
