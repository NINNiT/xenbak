#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeneralConfig {
    pub log_level: String,
}

impl Default for GeneralConfig {
    fn default() -> GeneralConfig {
        GeneralConfig {
            log_level: "info".into(),
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
pub struct MailMonitoringConfig {
    pub enabled: bool,
    pub smtp_server: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub smtp_to: Vec<String>,
}

impl Default for MailMonitoringConfig {
    fn default() -> MailMonitoringConfig {
        MailMonitoringConfig {
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
    pub mail: MailMonitoringConfig,
}

impl Default for MonitoringConfig {
    fn default() -> MonitoringConfig {
        MonitoringConfig {
            mail: MailMonitoringConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JobConfig {
    pub enabled: bool,
    pub name: String,
    pub schedule: String,
    pub storages: Vec<String>,
    pub tag_filter: Vec<String>,
    pub tag_filter_exclude: Vec<String>,
    pub timeout: u64,
    pub concurrency: u32,
    pub retention: u32,
    pub compression: String,
    pub limit_bandwidth: u32,
}

impl Default for JobConfig {
    fn default() -> JobConfig {
        JobConfig {
            enabled: false,
            name: String::default(),
            schedule: "0 0 * * *".into(),
            storages: vec![String::default()],
            tag_filter: vec![String::default()],
            tag_filter_exclude: vec![String::default()],
            timeout: 3600,
            concurrency: 1,
            retention: 7,
            compression: "none".into(),
            limit_bandwidth: 0,
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
