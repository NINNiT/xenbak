use std::str::FromStr;

use crate::{config::JobConfig, jobs::JobType, xapi::CompressionType};

pub mod borg;
pub mod local;

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
    pub compression: Option<Vec<CompressionType>>,
}

impl BackupObjectFilter {
    pub fn from_backup_object(backup_object: BackupObject) -> Self {
        BackupObjectFilter {
            job_type: Some(vec![backup_object.job_type]),
            vm_name: Some(vec![backup_object.vm_name]),
            time_stamp: Some((None, Some(backup_object.time_stamp))),
            compression: backup_object.compression.clone().map(|c| vec![c]),
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
    pub compression: Option<CompressionType>,
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
            compression,
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

        let compression = match parts[3].split('.').last() {
            Some(ext) => match CompressionType::from_extension(ext) {
                Ok(compression) => Some(compression),
                Err(_) => None,
            },
            _ => None,
        };

        Ok(BackupObject {
            job_type,
            xen_host: xen_host.to_string(),
            vm_name: vm_name.to_string(),
            time_stamp,
            size: None,
            compression,
        })
    }

    pub fn generate_name_without_extension(&self) -> String {
        format!(
            "{}__{}__{}__{}",
            self.xen_host.trim(),
            self.job_type.to_string(),
            self.vm_name.trim(),
            self.time_stamp.to_rfc3339()
        )
    }

    pub fn generate_name_with_extension(&self) -> String {
        let base_name = self.generate_name_without_extension();

        let base_extension = match self.job_type {
            JobType::VmBackup => "xva",
        };

        if self.compression.is_none() {
            return format!("{}.{}", base_name, base_extension);
        } else {
            return format!(
                "{}.{}.{}",
                base_name,
                base_extension,
                self.compression.as_ref().unwrap().to_extension()
            );
        };
    }
}

#[derive(Debug, Clone)]
pub enum StorageType {
    Local,
    Borg,
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
