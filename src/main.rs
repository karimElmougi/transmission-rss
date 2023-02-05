use transmission_rss::config::Config;
use transmission_rss::rss::process_feed;

use std::fs;
use std::path::PathBuf;

use clap::Parser;
use transmission_rpc::types::BasicAuth;
use transmission_rpc::TransClient;

/// Parse args
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the config file
    #[clap(short, long)]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::formatted_builder()
        .filter(None, log::LevelFilter::Warn)
        .filter(Some("transmission_rss"), log::LevelFilter::Info)
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or_default())
        .init();

    // Read env
    let args = Args::parse();

    // Read initial config file
    let file = match fs::read_to_string(&args.config) {
        Ok(val) => val,
        Err(err) => panic!("Failed to find file {:?}: {}", args.config, err),
    };
    let cfg: Config = toml::from_str(&file).unwrap();

    for feed in &cfg.rss_feeds {
        for rule in &feed.rules {
            let dir = cfg.base_download_dir.join(&rule.download_dir);
            if !dir.exists() {
                log::info!("Creating download_dir: `{}`", dir.to_string_lossy());
                std::fs::create_dir_all(dir)?;
            }
        }
    }

    // Open the database
    let db = kv::Store::open(&cfg.persistence.path)?;

    // Creates a new connection
    let basic_auth = BasicAuth {
        user: cfg.transmission.username.clone(),
        password: cfg.transmission.password.clone(),
    };
    let mut client = TransClient::with_auth(&cfg.transmission.url, basic_auth);

    for feed in cfg.rss_feeds {
        if let Err(err) = process_feed(&feed, &db, &mut client, &cfg.base_download_dir).await {
            log::error!("Error while processing feed `{}`: {err}", &feed.title);
        }
    }

    Ok(())
}
