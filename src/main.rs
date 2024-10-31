#![cfg(unix)]
use transmission_rss::config::Config;
use transmission_rss::{rss, transmission, Torrent};

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use home::home_dir;
use rustc_hash::FxHashMap;

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

    let links = db.load_map().with_context(|| "Unable to read link db")?;
    let retry_links = retry_db
        .load_map()
        .with_context(|| "Unable to read retry db")?;

    let torrents = fetch_new_torrents(&cfg, &links, &retry_links);

    let transmission_client = transmission::Client::new(&cfg);

    add_torrents(&transmission_client, &db, &retry_db, torrents);

    for (link, torrent) in retry_links {
        log::info!("Retrying {}", torrent.title);
        if let Err(err) = transmission_client.add(&torrent) {
            log::error!("{err}");
        } else if let Err(err) = retry_db.unset(&link) {
            log::error!("Unable to remove `{}` from retry.db: {err}", torrent.title);
        }
    }

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

fn fetch_new_torrents<'a>(
    cfg: &'a Config,
    links: &'a FxHashMap<String, String>,
    retry_links: &'a FxHashMap<String, Torrent>,
) -> impl Iterator<Item = Torrent> + 'a {
    cfg.rss_feeds
        .iter()
        .map(|feed| {
            rss::check_feed(feed)
                .inspect_err(|e| log::error!("Error checking feed `{}`: {e}", feed.name))
        })
        .filter_map(Result::ok)
        .flatten()
        .filter(|torrent| {
            !links.contains_key(&torrent.link) && !retry_links.contains_key(&torrent.link)
        })
}

fn add_torrents(
    client: &transmission::Client,
    db: &kv::Store<String>,
    retry_db: &kv::Store<Torrent>,
    torrents: impl Iterator<Item = Torrent>,
) {
    for torrent in torrents {
        log::info!("`{}` matches rule `{}`", torrent.title, torrent.rule);
        match client.add(&torrent) {
            Ok(()) => {
                if let Err(err) = db.set(&torrent.link, &torrent.title) {
                    log::error!(
                        "Failed to save link for `{}` into db: {err:?}",
                        torrent.title
                    );
                }
            }
            Err(err) => {
                log::error!("Failed to add torrent `{}`: {err}", torrent.title);
                if let Err(err) = retry_db.set(&torrent.link, &torrent) {
                    log::error!(
                        "Failed to save link for `{}` into retry.db: {err}",
                        torrent.title
                    );
                }
            }
        }
    }
}
