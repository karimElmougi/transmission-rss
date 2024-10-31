pub mod config;
pub mod rss;
pub mod transmission;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Torrent {
    pub link: String,
    pub title: String,
    pub download_dir: PathBuf,
    pub labels: Vec<String>,
}
