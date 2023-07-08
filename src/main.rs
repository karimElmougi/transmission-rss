#![cfg(unix)]
use transmission_rss::config::Config;
use transmission_rss::{rss, transmission};

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

    let retry_db_path = config_dir.join("retry.db");
    let retry_db = kv::Store::open(&retry_db_path)
        .with_context(|| format!("Unable to open retry file {retry_db_path:?}"))?;

    let transmission_client = transmission::Client::new(&cfg, db.clone(), retry_db.clone());

    let rss_client = rss::Client::new(db, retry_db);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let fetch_tasks = cfg
        .rss_feeds
        .into_iter()
        .map(|feed| rss_client.check_feed(feed));

    let add_tasks = runtime
        .block_on(join_all(fetch_tasks))
        .into_iter()
        .flatten()
        .map(|torrent| transmission_client.add(torrent));

    runtime.block_on(join_all(add_tasks));
    runtime.block_on(transmission_client.retry_missing());

    Ok(())
}

fn load_config(path: &Path) -> Result<Config> {
    let file =
        fs::read_to_string(path).with_context(|| format!("Failed to open config file {path:?}"))?;

    toml::from_str(&file).context("Config file is invalid")
}

fn config_dir_path() -> Result<PathBuf> {
    let mut path = home_dir().context("Unable to locate use home directory, is $HOME set?")?;
    path.push(".config/transmission-rss");
    Ok(path)
}
