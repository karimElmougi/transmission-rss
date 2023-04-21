#![cfg(unix)]
use transmission_rss::config::Config;
use transmission_rss::{rss, TransmissionClient};

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use futures::future::join_all;
use home::home_dir;

fn main() -> Result<()> {
    pretty_env_logger::formatted_builder()
        .filter(None, log::LevelFilter::Warn)
        .filter(Some("transmission_rss"), log::LevelFilter::Info)
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or_default())
        .init();

    let config_dir = config_dir_path()?;

    let config_path = config_dir.join("config.toml");
    let cfg = load_config(&config_path)?;

    let db_path = config_dir.join("links.db");
    let db = kv::Store::open(&db_path)
        .with_context(|| format!("Unable to open persistence file {db_path:?}"))?;

    let transmission_client = TransmissionClient::new(&cfg, db.clone());

    let rss_client = rss::Client::new(db.clone(), cfg.base_download_dir);

    let fetch_tasks = cfg
        .rss_feeds
        .into_iter()
        .map(|feed| rss_client.check_feed(feed));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let add_tasks = runtime
        .block_on(join_all(fetch_tasks))
        .into_iter()
        .flatten()
        .map(|torrent| transmission_client.add(torrent));

    runtime.block_on(join_all(add_tasks));

    Ok(())
}

fn load_config(path: &Path) -> Result<Config> {
    let file =
        fs::read_to_string(path).with_context(|| format!("Failed to open config file {path:?}"))?;

    let mut cfg: Config = toml::from_str(&file).context("Config file is invalid")?;

    for feed in &mut cfg.rss_feeds {
        // Only keep rules with a valid download directory
        feed.rules.retain(|rule| {
            let dir = cfg.base_download_dir.join(&rule.download_dir);
            if let Err(err) = ensure_exists(&dir) {
                log::error!("{err}");
                false
            } else {
                true
            }
        });
    }

    Ok(cfg)
}

fn config_dir_path() -> Result<PathBuf> {
    let mut path = home_dir().context("Unable to locate use home directory, is $HOME set?")?;
    path.push(".config/transmission-rss");
    Ok(path)
}

fn ensure_exists(dir: &Path) -> Result<()> {
    let exists = dir
        .try_exists()
        .with_context(|| format!("Couldn't access directory {dir:?}"))?;

    if !exists {
        log::info!("Creating directory {dir:?}");
        std::fs::create_dir_all(dir)
            .with_context(|| format!("Unable to create directory {dir:?}"))?;
    }

    Ok(())
}
