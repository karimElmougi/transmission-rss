use transmission_rss::config::Config;
use transmission_rss::rss;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use futures::future::join_all;
use transmission_rpc::types::BasicAuth;
use transmission_rpc::SharableTransClient;

/// Parse args
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the config file
    #[clap(short, long)]
    config: PathBuf,
}

fn main() -> Result<()> {
    pretty_env_logger::formatted_builder()
        .filter(None, log::LevelFilter::Warn)
        .filter(Some("transmission_rss"), log::LevelFilter::Info)
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or_default())
        .init();

    let args = Args::parse();

    let cfg = load_config(&args.config)?;

    // Open the database
    let db = kv::Store::open(&cfg.persistence.path)
        .with_context(|| format!("Unable to open persistence file {:?}", cfg.persistence.path))?;

    // Creates a new connection
    let basic_auth = BasicAuth {
        user: cfg.transmission.username.clone(),
        password: cfg.transmission.password.clone(),
    };
    let client = SharableTransClient::with_auth(cfg.transmission.url, basic_auth);

    let tasks = cfg
        .rss_feeds
        .into_iter()
        .map(|feed| rss::check_feed(feed, &db, &cfg.base_download_dir, &client));

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(join_all(tasks));

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
