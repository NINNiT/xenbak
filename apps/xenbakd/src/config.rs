#![allow(dead_code)]
use serde::{de::IntoDeserializer, Deserialize, Serialize};

use crate::xapi::CompressionType;

// deserialize "none" string for Option<SomeEnum>, e.g. for Option<CompressionType>. make it work for any source, not just JSON
//e.g. the toml line compression = "none"             # Compression type:  gzip, zstd or none
pub fn deserialize_option_enum<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    if s == "none" {
        Ok(None)
    } else {
        Ok(Some(serde::Deserialize::deserialize(
            s.into_deserializer(),
        )?))
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeneralConfig {
    pub log_level: String,
    pub hostname: String,
}

impl Default for GeneralConfig {
    fn default() -> GeneralConfig {
        GeneralConfig {
            log_level: "info".into(),
            hostname: "localhost".into(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LocalStorageConfig {
    pub enabled: bool,
    pub name: String,
    pub path: String,
}

impl Default for LocalStorageConfig {
    fn default() -> LocalStorageConfig {
        LocalStorageConfig {
            enabled: false,
            name: String::default(),
            path: String::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StorageConfig {
    pub local: Vec<LocalStorageConfig>,
}

impl Default for StorageConfig {
    fn default() -> StorageConfig {
        StorageConfig {
            local: vec![LocalStorageConfig::default()],
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HealthchecksConfig {
    pub enabled: bool,
    pub api_key: String,
    pub server: String,
    pub grace: u64,
    pub max_retries: u32,
}

impl Default for HealthchecksConfig {
    fn default() -> HealthchecksConfig {
        HealthchecksConfig {
            enabled: false,
            api_key: String::default(),
            server: "https://hc-ping.com".into(),
            grace: 7200,
            max_retries: 3,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MailConfig {
    pub enabled: bool,
    pub smtp_server: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub smtp_to: Vec<String>,
}

impl Default for MailConfig {
    fn default() -> MailConfig {
        MailConfig {
            enabled: false,
            smtp_server: String::default(),
            smtp_port: 587,
            smtp_user: String::default(),
            smtp_password: String::default(),
            smtp_from: String::default(),
            smtp_to: vec![String::default()],
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MonitoringConfig {
    pub mail: MailConfig,
    pub healthchecks: HealthchecksConfig,
}

impl Default for MonitoringConfig {
    fn default() -> MonitoringConfig {
        MonitoringConfig {
            mail: MailConfig::default(),
            healthchecks: HealthchecksConfig::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobConfig {
    pub enabled: bool,
    pub name: String,
    pub schedule: String,
    pub tag_filter: Vec<String>,
    pub tag_filter_exclude: Vec<String>,
    pub concurrency: u32,
    pub retention: u32,
    #[serde(deserialize_with = "deserialize_option_enum")]
    pub compression: Option<CompressionType>,
}

impl Default for JobConfig {
    fn default() -> JobConfig {
        JobConfig {
            enabled: false,
            name: String::default(),
            schedule: "0 0 * * *".into(),
            tag_filter: vec![String::default()],
            tag_filter_exclude: vec![String::default()],
            concurrency: 1,
            retention: 7,
            compression: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub storage: StorageConfig,
    pub monitoring: MonitoringConfig,
    pub jobs: Vec<JobConfig>,
}

impl Default for AppConfig {
    fn default() -> AppConfig {
        AppConfig {
            general: GeneralConfig::default(),
            storage: StorageConfig::default(),
            monitoring: MonitoringConfig::default(),
            jobs: vec![JobConfig::default()],
        }
    }
}
