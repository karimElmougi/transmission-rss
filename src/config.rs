use std::fs::read_to_string;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub base_download_dir: PathBuf,
    pub persistence: Persistence,
    pub transmission: Transmission,
    pub rss_feeds: Vec<RssFeed>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Persistence {
    pub path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(try_from = "RawTransmission")]
pub struct Transmission {
    pub url: String,
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
    pub url: String,
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
    pub title: String,
    pub url: String,
    pub rules: Vec<DownloadRule>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DownloadRule {
    pub filter: String,
    pub download_dir: PathBuf,
}
