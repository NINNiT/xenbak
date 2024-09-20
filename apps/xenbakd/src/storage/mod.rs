use crate::{config::JobConfig, jobs::JobType};

pub mod borg;
pub mod local;

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

pub trait CompressionType: Sized {
    fn to_extension(&self) -> String;
    fn from_extension(extension: &str) -> eyre::Result<Self>;
    fn to_cli_arg(&self) -> String;
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
    pub xen_host: Option<Vec<String>>,
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
            xen_host: Some(vec![backup_object.xen_host]),
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
        _size: Option<u64>,
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
}

#[derive(Debug, Clone)]
pub enum StorageType {
    Local,
    Borg,
}
