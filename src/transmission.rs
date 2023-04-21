use crate::config::Config;
use crate::{Torrent, TIMEOUT};

use anyhow::{anyhow, Result};
use tokio::time::error::Elapsed;
use tokio::time::timeout;
use transmission_rpc::types::{BasicAuth, TorrentAddArgs};
use transmission_rpc::SharableTransClient;

pub struct Client {
    inner: SharableTransClient,
    db: kv::Store<String>,
    retry_db: kv::Store<Torrent>,
}

impl Client {
    pub fn new(cfg: &Config, db: kv::Store<String>, retry_db: kv::Store<Torrent>) -> Self {
        let basic_auth = BasicAuth {
            user: cfg.transmission.username.clone(),
            password: cfg.transmission.password.clone(),
        };
        let inner = SharableTransClient::with_auth(cfg.transmission.url.clone(), basic_auth);

        Self {
            inner,
            db,
            retry_db,
        }
    }

    pub async fn retry_missing(&self) {
        let torrents_to_retry = match self.retry_db.load_map() {
            Ok(t) => t,
            Err(err) => {
                log::error!("Unable to read retry.db: {err}");
                return;
            }
        };

        for (link, torrent) in torrents_to_retry {
            if let Err(err) = flatten(timeout(TIMEOUT, self.add_impl(&torrent)).await) {
                log::error!("{err}");
            } else if let Err(err) = self.retry_db.unset(&link) {
                log::error!("Unable to remove `{}` from retry.db: {err}", torrent.title);
            }
        }
    }

    pub async fn add(&self, torrent: Torrent) {
        if let Err(err) = flatten(timeout(TIMEOUT, self.add_impl(&torrent)).await) {
            log::error!("{err}");
            if let Err(err) = async { self.retry_db.set(&torrent.link, &torrent) }.await {
                log::error!(
                    "Failed to save link for `{}` into retry.db: {err:?}",
                    torrent.title
                );
            }
        }
    }

    async fn add_impl(&self, torrent: &Torrent) -> Result<()> {
        let add: TorrentAddArgs = TorrentAddArgs {
            filename: Some(torrent.link.to_string()),
            download_dir: Some(torrent.download_dir.to_string_lossy().to_string()),
            labels: Some(torrent.labels.clone()),
            ..TorrentAddArgs::default()
        };

        let response = match self.inner.torrent_add(add).await {
            Ok(response) => response,
            Err(err) => {
                return Err(anyhow!("Error connecting to Transmission: {err}"));
            }
        };

        if response.is_ok() {
            if let Err(err) = async { self.db.set(&torrent.link, &torrent.title) }.await {
                return Err(anyhow!(
                    "Failed to save link for `{}` into db: {err:?}",
                    torrent.title
                ));
            }
        } else {
            let reason = response.result;
            return Err(anyhow!(
                "Failed to add torrent `{}`: {reason}",
                torrent.title
            ));
        }

        Ok(())
    }
}

fn flatten<T>(r: Result<Result<T>, Elapsed>) -> Result<T> {
    match r {
        Ok(Ok(t)) => Ok(t),
        Ok(Err(err)) => Err(err),
        Err(_) => Err(anyhow!("Timeout while adding torrent")),
    }
}
