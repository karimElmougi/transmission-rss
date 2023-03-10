use std::fs::read_to_string;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub base_download_dir: PathBuf,
    pub transmission: Transmission,
    pub rss_feeds: Vec<RssFeed>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(try_from = "RawTransmission")]
pub struct Transmission {
    pub url: Url,
    pub username: String,
    pub password: String,
}

impl TryFrom<RawTransmission> for Transmission {
    type Error = std::io::Error;

    fn try_from(value: RawTransmission) -> Result<Self, Self::Error> {
        let password = match value.password {
            TransmissionPassword::Raw { password } => password,
            TransmissionPassword::File { password_file } => {
                read_to_string(password_file)?.trim().to_string()
            }
        };
        Ok(Transmission {
            url: value.url,
            username: value.username,
            password,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RawTransmission {
    pub url: Url,
    pub username: String,
    #[serde(flatten)]
    pub password: TransmissionPassword,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum TransmissionPassword {
    Raw { password: String },
    File { password_file: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RssFeed {
    pub name: String,
    pub url: Url,
    pub rules: Vec<DownloadRule>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DownloadRule {
    pub filter: String,
    pub download_dir: PathBuf,
}

impl DownloadRule {
    pub fn check(&self, title: &str) -> bool {
        self.filter
            .split_whitespace()
            .all(|word| title.contains(word))
    }
}
