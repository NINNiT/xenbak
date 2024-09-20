use serde::{Deserialize, Serialize};

use crate::config::{BorgLocalStorageConfig, JobConfig};

use super::{BackupObjectFilter, StorageHandler, StorageStatus, StorageType};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum BorgCompressionType {
    #[serde(rename = "lz4")]
    LZ4,
    #[serde(rename = "zstd")]
    Zstd,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum BorgEncryptionType {
    #[serde(rename = "repokey")]
    Repokey,
    #[serde(rename = "repokey-blake2")]
    RepokeyBlake2,
}

#[derive(Debug, Clone)]
pub struct BorgStorage {
    pub storage_type: StorageType,
    pub storage_config: BorgLocalStorageConfig,
    pub job_config: JobConfig,
}

impl BorgStorage {
    pub fn new(storage_config: BorgLocalStorageConfig, job_config: JobConfig) -> Self {
        BorgStorage {
            storage_type: StorageType::Borg,
            job_config,
            storage_config,
        }
    }
}

#[async_trait::async_trait]
impl StorageHandler for BorgStorage {
    async fn status(&self) -> eyre::Result<StorageStatus> {
        todo!()
    }

    fn get_job_config(&self) -> JobConfig {
        self.job_config.clone()
    }

    fn get_storage_type(&self) -> StorageType {
        self.storage_type.clone()
    }

    async fn initialize(&self) -> eyre::Result<()> {
        todo!()
    }

    async fn list(
        &self,
        _filter: BackupObjectFilter,
    ) -> eyre::Result<Vec<crate::storage::BackupObject>> {
        todo!()
    }

    async fn rotate(&self, _filter: BackupObjectFilter) -> eyre::Result<()> {
        todo!()
    }

    async fn handle_stdio_stream(
        &self,
        _backup_object: crate::storage::BackupObject,
        _stdout_stream: tokio::process::ChildStdout,
        _stderr_stream: tokio::process::ChildStderr,
    ) -> eyre::Result<()> {
        Ok(())
    }
}
