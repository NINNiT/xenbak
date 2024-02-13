use chrono::Utc;
use serde::{Deserialize, Serialize};

use self::error::XApiParseError;

pub mod cli;
pub mod error;

pub fn parse_timestamp(timestamp: &str) -> Result<chrono::DateTime<chrono::Utc>, XApiParseError> {
    let naive = chrono::NaiveDateTime::parse_from_str(timestamp, "%Y%m%dT%H:%M:%S%Z")?;
    let utc = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, Utc);
    Ok(utc)
}

pub type UUID = String;
pub type UUIDs = Vec<UUID>;

#[derive(Debug, Default)]
pub struct VM {
    pub uuid: String,
    pub name_label: String,
    pub name_description: String,
    pub is_a_template: bool,
    pub is_default_template: bool,
    pub is_a_snapshot: bool,
    pub snapshot_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub enum SnapshotType {
    Normal,
    Memory,
}

impl Default for SnapshotType {
    fn default() -> Self {
        Self::Normal
    }
}

impl ToString for SnapshotType {
    fn to_string(&self) -> String {
        match self {
            SnapshotType::Normal => "basic".to_string(),
            SnapshotType::Memory => "memory".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum CompressionType {
    #[serde(rename = "gzip")]
    Gzip,
    #[serde(rename = "zstd")]
    Zstd,
}

impl CompressionType {
    pub fn to_extension(&self) -> String {
        match self {
            CompressionType::Gzip => "gz".to_string(),
            CompressionType::Zstd => "zst".to_string(),
        }
    }

    pub fn from_extension(extension: &str) -> eyre::Result<CompressionType> {
        match extension {
            "gz" => Ok(CompressionType::Gzip),
            "zst" => Ok(CompressionType::Zstd),
            _ => Err(eyre::eyre!("Invalid compression extension")),
        }
    }

    pub fn to_cli_arg(&self) -> String {
        match self {
            CompressionType::Gzip => "gzip".to_string(),
            CompressionType::Zstd => "zstd".to_string(),
        }
    }
}