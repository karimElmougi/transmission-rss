pub mod config;
pub mod rss;
pub mod transmission;

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Serialize, Deserialize)]
pub struct Torrent {
    pub link: String,
    pub title: String,
    pub download_dir: PathBuf,
    pub labels: Vec<String>,
}
