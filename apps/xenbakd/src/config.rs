#![allow(dead_code)]
use serde::{de::IntoDeserializer, Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    storage::{
        self,
        borg::{BorgCompressionType, BorgEncryptionType},
        StorageHandler,
    },
    xapi::CompressionType,
};

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
    #[serde(deserialize_with = "deserialize_option_enum")]
    pub compression: Option<CompressionType>,
    pub retention: u32,
}

impl Default for LocalStorageConfig {
    fn default() -> LocalStorageConfig {
        LocalStorageConfig {
            enabled: false,
            name: String::default(),
            path: String::default(),
            compression: None,
            retention: 7,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BorgLocalStorageConfig {
    pub enabled: bool,
    pub name: String,
    pub repo_base_path: String,
    #[serde(deserialize_with = "deserialize_option_enum")]
    pub encryption: Option<BorgEncryptionType>,
    #[serde(deserialize_with = "deserialize_option_enum")]
    pub compression: Option<BorgCompressionType>,
    pub retention: u32,
    pub temp_dir: String,
}

impl Default for BorgLocalStorageConfig {
    fn default() -> BorgLocalStorageConfig {
        BorgLocalStorageConfig {
            enabled: false,
            name: String::default(),
            repo_base_path: String::default(),
            encryption: None,
            compression: None,
            retention: 7,
            temp_dir: "/tmp/xenbakd".into(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StorageConfig {
    pub local: Vec<LocalStorageConfig>,
    pub borg_local: Vec<BorgLocalStorageConfig>,
}

impl Default for StorageConfig {
    fn default() -> StorageConfig {
        StorageConfig {
            local: vec![LocalStorageConfig::default()],
            borg_local: vec![BorgLocalStorageConfig::default()],
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
    pub storages: Vec<String>,
    pub xen_hosts: Vec<String>,
}

impl JobConfig {
    pub fn get_storages(&self, config: StorageConfig) -> Vec<Arc<dyn StorageHandler>> {
        let mut storages: Vec<Arc<dyn StorageHandler>> = Vec::new();

        let local_storage = config
            .local
            .iter()
            .filter(|x| x.enabled && self.storages.contains(&x.name))
            .map(|x| {
                Arc::new(storage::local::LocalStorage::new(x.clone(), self.clone()))
                    as Arc<dyn StorageHandler>
            })
            .collect::<Vec<Arc<dyn StorageHandler>>>();

        let borg_local_storage = config
            .borg_local
            .iter()
            .filter(|x| x.enabled && self.storages.contains(&x.name))
            .map(|x| {
                Arc::new(storage::borg::BorgStorage::new(x.clone(), self.clone()))
                    as Arc<dyn StorageHandler>
            })
            .collect::<Vec<Arc<dyn StorageHandler>>>();

        storages.extend(local_storage);
        storages.extend(borg_local_storage);

        storages
    }

    pub fn get_xen_configs(&self, xen_config: Vec<XenConfig>) -> Vec<XenConfig> {
        xen_config
            .iter()
            .filter(|x| self.xen_hosts.contains(&x.name))
            .cloned()
            .collect()
    }
}

impl Default for JobConfig {
    fn default() -> JobConfig {
        JobConfig {
            enabled: false,
            name: String::default(),
            schedule: "0 0 * * *".into(),
            tag_filter: vec![String::default()],
            tag_filter_exclude: vec![String::default()],
            xen_hosts: vec![String::default()],
            storages: vec![String::default()],
            concurrency: 1,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Hash, Eq)]
pub struct XenConfig {
    pub enabled: bool,
    pub name: String,
    pub username: String,
    pub server: String,
    pub password: String,
    pub port: u16,
}

impl Default for XenConfig {
    fn default() -> XenConfig {
        XenConfig {
            enabled: false,
            name: "127.0.0.1".into(),
            username: String::default(),
            server: "127.0.0.1".into(),
            password: String::default(),
            port: 443,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub xen: Vec<XenConfig>,
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
            xen: vec![XenConfig {
                enabled: false,
                name: String::default(),
                username: String::default(),
                server: String::default(),
                password: String::default(),
                port: 443,
            }],
        }
    }
}
