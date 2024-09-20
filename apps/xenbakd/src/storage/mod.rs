use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{config::JobConfig, jobs::JobType};

pub mod borg;
pub mod local;

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

#[derive(Debug, Clone)]
pub struct StorageStatus {
    pub free_space: u64,
    pub total_space: u64,
    pub used_space: u64,
    pub backup_count: u32,
}

#[derive(Debug, Clone)]
pub struct BackupObjectFilter {
    pub job_type: Option<Vec<JobType>>,
    pub vm_name: Option<Vec<String>>,
    pub time_stamp: Option<(
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
    )>,
}

impl BackupObjectFilter {
    pub fn from_backup_object(backup_object: BackupObject) -> Self {
        BackupObjectFilter {
            job_type: Some(vec![backup_object.job_type]),
            vm_name: Some(vec![backup_object.vm_name]),
            time_stamp: Some((None, Some(backup_object.time_stamp))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BackupObject {
    pub job_type: JobType,
    pub vm_name: String,
    pub xen_host: String,
    pub time_stamp: chrono::DateTime<chrono::Utc>,
    pub size: Option<u64>,
}

impl BackupObject {
    pub fn new(
        job_type: JobType,
        vm_name: String,
        xen_host: String,
        time_stamp: chrono::DateTime<chrono::Utc>,
        compression: Option<CompressionType>,
    ) -> Self {
        BackupObject {
            job_type,
            vm_name,
            xen_host,
            time_stamp,
            size: None,
        }
    }

    pub fn to_filter(&self) -> BackupObjectFilter {
        BackupObjectFilter::from_backup_object(self.clone())
    }

    // vm__debian-03__2024-02-09T10:19:02+00:00.xva.gz. compression extension might be missing
    pub async fn from_name_with_extension(filename: String) -> eyre::Result<BackupObject> {
        let parts: Vec<&str> = filename.split("__").collect();
        if parts.len() != 4 {
            return Err(eyre::eyre!("Invalid backup object name"));
        }

        // vm__debian-01__2024-02-09T11:28:01+00:00.xva.gz

        let xen_host = parts[0];
        let job_type = JobType::from_str(parts[1])?;
        let vm_name = parts[2];
        let time_stamp =
            chrono::DateTime::parse_from_rfc3339(parts[3].split(".").next().unwrap())?.to_utc();

        Ok(BackupObject {
            job_type,
            xen_host: xen_host.to_string(),
            vm_name: vm_name.to_string(),
            time_stamp,
            size: None,
        })
    }
}

#[derive(Debug, Clone)]
pub enum StorageType {
    Local,
    BorgLocal,
}

#[async_trait::async_trait]
pub trait StorageHandler: Send + Sync {
    fn get_storage_type(&self) -> StorageType;
    fn get_job_config(&self) -> JobConfig;
    async fn status(&self) -> eyre::Result<StorageStatus>;
    async fn initialize(&self) -> eyre::Result<()>;
    async fn list(&self, filter: BackupObjectFilter) -> eyre::Result<Vec<BackupObject>>;
    async fn rotate(&self, filter: BackupObjectFilter) -> eyre::Result<()>;
    async fn handle_stdio_stream(
        &self,
        backup_object: BackupObject,
        stdout_stream: tokio::process::ChildStdout,
        stderr_stream: tokio::process::ChildStderr,
    ) -> eyre::Result<()>;
}
