use crate::config::Config;
use crate::Torrent;

use std::path::PathBuf;
use std::time::Duration;

use thiserror::Error;
use tokio::time::timeout;
use transmission_rpc::types::{BasicAuth, TorrentAddArgs};
use transmission_rpc::SharableTransClient;

#[derive(Error, Debug)]
pub enum Error {
    #[error("error connecting to Transmission: {0}")]
    Connection(String),
    #[error("Transmission RPC error: {0}")]
    TransmissionRpc(String),
    #[error("connection timed out")]
    Timeout,
}

pub struct Client {
    inner: SharableTransClient,
    base_download_dir: PathBuf,
    runtime: tokio::runtime::Runtime,
}

impl Client {
    pub fn new(cfg: &Config) -> Self {
        let basic_auth = BasicAuth {
            user: cfg.transmission.username.clone(),
            password: cfg.transmission.password.clone(),
        };
        let inner = SharableTransClient::with_auth(cfg.transmission.url.clone(), basic_auth);

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        Self {
            inner,
            runtime,
            base_download_dir: cfg.base_download_dir.clone(),
        }
    }

    pub fn add(&self, torrent: &Torrent) -> Result<(), Error> {
        let download_dir = self.base_download_dir.join(&torrent.download_dir);

        let add: TorrentAddArgs = TorrentAddArgs {
            filename: Some(torrent.link.to_string()),
            download_dir: Some(download_dir.to_string_lossy().to_string()),
            labels: Some(torrent.labels.clone()),
            ..TorrentAddArgs::default()
        };

        const TIMEOUT: Duration = Duration::from_secs(5);
        let response = self
            .runtime
            .block_on(timeout(TIMEOUT, self.inner.torrent_add(add)))
            .map_err(|_| Error::Timeout)?
            .map_err(|err| Error::Connection(err.to_string()))?;

        if response.is_ok() {
            Ok(())
        } else {
            let reason = response.result;
            Err(Error::TransmissionRpc(format!(
                "Failed to add torrent `{}`: {reason}",
                torrent.title
            )))
        }
    }
}
