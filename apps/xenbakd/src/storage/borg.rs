use std::{path::PathBuf, str::FromStr};

use async_tempfile::TempFile;
use eyre::Context;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tracing::{debug, info};

use tokio::process::Command as AsyncCommand;

use crate::{
    config::{BorgStorageConfig, JobConfig},
    jobs::JobType,
};

use super::{BackupObjectFilter, CompressionType, StorageHandler, StorageStatus, StorageType};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum BorgCompressionType {
    #[serde(rename = "lz4")]
    LZ4,
    #[serde(rename = "zstd")]
    Zstd,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BorgStorageRetention {
    pub daily: u32,
    pub weekly: u32,
    pub monthly: u32,
    pub yearly: u32,
}

impl CompressionType for BorgCompressionType {
    fn to_extension(&self) -> String {
        match self {
            BorgCompressionType::LZ4 => "lz4".to_string(),
            BorgCompressionType::Zstd => "zst".to_string(),
        }
    }

    fn from_extension(extension: &str) -> eyre::Result<BorgCompressionType> {
        match extension {
            "lz4" => Ok(BorgCompressionType::LZ4),
            "zst" => Ok(BorgCompressionType::Zstd),
            _ => Err(eyre::eyre!("Invalid compression extension")),
        }
    }

    fn to_cli_arg(&self) -> String {
        match self {
            BorgCompressionType::LZ4 => "lz4".to_string(),
            BorgCompressionType::Zstd => "zstd".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum BorgEncryptionType {
    #[serde(rename = "repokey")]
    Repokey,
    #[serde(rename = "repokey-blake2")]
    RepokeyBlake2,
}

impl ToString for BorgEncryptionType {
    fn to_string(&self) -> String {
        match self {
            BorgEncryptionType::Repokey => "repokey".to_string(),
            BorgEncryptionType::RepokeyBlake2 => "repokey-blake2".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BorgLocalStorage {
    pub storage_type: StorageType,
    pub storage_config: BorgStorageConfig,
    pub job_config: JobConfig,
}

impl BorgLocalStorage {
    pub fn new(storage_config: BorgStorageConfig, job_config: JobConfig) -> Self {
        BorgLocalStorage {
            storage_type: StorageType::Borg,
            job_config,
            storage_config,
        }
    }

    pub fn backup_object_to_archive_name(
        &self,
        backup_object: crate::storage::BackupObject,
    ) -> String {
        format!(
            "{}__{}__{}__{}",
            backup_object.xen_host,
            backup_object.job_type.to_string(),
            backup_object.vm_name,
            backup_object.time_stamp.to_rfc3339()
        )
    }

    pub fn _archive_name_to_backup_object(
        &self,
        archive_name: String,
    ) -> crate::storage::BackupObject {
        let parts: Vec<&str> = archive_name.split("__").collect();
        if parts.len() != 4 {
            panic!("Invalid backup object name");
        }

        let xen_host = parts[0];
        let job_type = JobType::from_str(parts[1]).unwrap();
        let vm_name = parts[2];
        let time_stamp = chrono::DateTime::parse_from_rfc3339(parts[3].split(".").next().unwrap())
            .unwrap()
            .to_utc();

        crate::storage::BackupObject {
            job_type,
            xen_host: xen_host.to_string(),
            vm_name: vm_name.to_string(),
            time_stamp,
            size: None,
        }
    }

    pub fn get_rsh_env(&self) -> Option<String> {
        if let Some(ssh_key_path) = &self.storage_config.ssh_key_path {
            Some(format!(
                "ssh -o StrictHostKeyChecking=no -i {}",
                ssh_key_path
            ))
        } else {
            None
        }
    }

    pub fn borg_base_cmd(&self) -> AsyncCommand {
        let mut cmd = AsyncCommand::new("borg");
        cmd.env("BORG_REPO", self.storage_config.repository.clone());
        cmd.env("BORG_UNKNOWN_UNENCRYPTED_REPO_ACCESS_IS_OK", "yes");
        if let Some(rsh) = self.get_rsh_env() {
            cmd.env("BORG_RSH", rsh);
        }
        cmd.arg("--lock-wait").arg("300");
        cmd
    }
}

#[async_trait::async_trait]
impl StorageHandler for BorgLocalStorage {
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
        let span = tracing::span!(tracing::Level::DEBUG, "BorgLocalStorage::initialize");
        let _enter = span.enter();

        let temp_dir_result: eyre::Result<()> = async {
            tokio::fs::create_dir_all(&self.storage_config.temp_dir)
                .await
                .wrap_err("Failed to create temporary directory for borg storage")?;

            Ok(())
        }
        .await;

        if let Err(e) = temp_dir_result {
            return Err(e);
        }

        let borg_init_result: eyre::Result<()> = async {
            let mut init_cmd = self.borg_base_cmd();
            init_cmd.arg("init");

            init_cmd
                .arg("--encryption")
                .arg(match &self.storage_config.encryption {
                    Some(encryption) => encryption.to_string(),
                    None => "none".to_string(),
                });

            let init_output = init_cmd.output().await?;

            if !init_output.status.success() {
                // check if it errors due to repo already existing -> that one is okay
                if String::from_utf8_lossy(&init_output.stderr)
                    .contains("repository already exists")
                {
                    debug!("Borg repository already exists, skipping init");
                    return Ok(());
                }

                return Err(eyre::eyre!(
                    "Failed to initialize borg repository: {}",
                    String::from_utf8_lossy(&init_output.stderr)
                ));
            };
            Ok(())
        }
        .await;

        if let Err(e) = borg_init_result {
            return Err(e);
        }

        borg_init_result
    }

    async fn list(
        &self,
        _filter: BackupObjectFilter,
    ) -> eyre::Result<Vec<crate::storage::BackupObject>> {
        todo!()
    }

    async fn rotate(&self, filter: BackupObjectFilter) -> eyre::Result<()> {
        if self.storage_config.retention.daily == 0
            && self.storage_config.retention.weekly == 0
            && self.storage_config.retention.monthly == 0
            && self.storage_config.retention.yearly == 0
        {
            info!("Retention is set to 0, skipping rotation...");
            return Ok(());
        }

        let mut prune_cmd = self.borg_base_cmd();
        prune_cmd.arg("prune");

        prune_cmd
            .arg("--keep-daily")
            .arg(self.storage_config.retention.daily.to_string().as_str());

        prune_cmd
            .arg("--keep-weekly")
            .arg(self.storage_config.retention.weekly.to_string().as_str());

        prune_cmd
            .arg("--keep-monthly")
            .arg(self.storage_config.retention.monthly.to_string().as_str());

        prune_cmd
            .arg("--keep-yearly")
            .arg(self.storage_config.retention.yearly.to_string().as_str());

        prune_cmd.arg("--glob-archives").arg(format!(
            "{}__{}__{}*",
            filter
                .xen_host
                .unwrap_or_default()
                .first()
                .unwrap_or(&"".to_string()),
            filter
                .job_type
                .unwrap_or_default()
                .first()
                .unwrap_or(&JobType::VmBackup)
                .to_string(),
            filter
                .vm_name
                .unwrap_or_default()
                .first()
                .unwrap_or(&"".to_string())
        ));

        info!("Pruning borg repository...");
        let prune_output = prune_cmd.output().await?;

        if !prune_output.status.success() {
            return Err(eyre::eyre!(
                "Failed to prune borg repository: {}",
                String::from_utf8_lossy(&prune_output.stderr)
            ));
        }

        info!("Compacting borg repository...");
        let mut compact_cmd = self.borg_base_cmd();
        compact_cmd.arg("compact");

        let compact_output = compact_cmd.output().await?;

        if !compact_output.status.success() {
            return Err(eyre::eyre!(
                "Failed to compact borg repository: {}",
                String::from_utf8_lossy(&compact_output.stderr)
            ));
        }

        Ok(())
    }

    async fn handle_stdio_stream(
        &self,
        backup_object: crate::storage::BackupObject,
        mut stdout_stream: tokio::process::ChildStdout,
        mut stderr_stream: tokio::process::ChildStderr,
    ) -> eyre::Result<()> {
        let mut temp_file = TempFile::new_in(PathBuf::from(&self.storage_config.temp_dir))
            .await
            .wrap_err("Failed to create temporary file for borg backup stream")?;

        let tempfile_results = async {
            debug!(
                "Writing export stream to temporary file {}...",
                temp_file.file_path().clone().as_os_str().to_string_lossy()
            );

            const BUFFER_SIZE: usize = 1024 * 1024 * 10;
            let mut stdout_buffered = tokio::io::BufReader::with_capacity(BUFFER_SIZE, &mut stdout_stream);
            let mut stderr_buffered = tokio::io::BufReader::new(&mut stderr_stream);
            let tempfile_copy = tokio::io::copy(&mut stdout_buffered, &mut temp_file).await?;

            debug!("Wrote {} bytes to temporary file", tempfile_copy);

            let mut stderr = Vec::new();
            stderr_buffered.read_to_end(&mut stderr).await?;
            if !stderr.is_empty() {
                let stderr = String::from_utf8_lossy(&stderr);
                return Err(eyre::eyre!(
                    "Error encountered in stderr output: {}",
                    stderr
                ));
            }

            Ok(temp_file)
        }
        .await.wrap_err(
            "Failed to write export stream to temporary file, or encountered error in stderr output",
            );

        let borg_results = async {
            let temp_file = tempfile_results?;

            info!(
                "Running borg backup to repo {} with archive: {}",
                self.storage_config.repository,
                self.backup_object_to_archive_name(backup_object.clone())
            );

            let mut borg_cmd = self.borg_base_cmd();
            borg_cmd.arg("create");

            if let Some(compression) = &self.storage_config.compression {
                borg_cmd.arg("--compression").arg(compression.to_cli_arg());
            }

            borg_cmd.arg(
                format!(
                    "::{}",
                    self.backup_object_to_archive_name(backup_object.clone())
                )
                .as_str(),
            );

            borg_cmd.arg(
                temp_file
                    .file_path()
                    .clone()
                    .as_os_str()
                    .to_string_lossy()
                    .to_string(),
            );

            let borg_output = borg_cmd.output().await?;

            if !borg_output.status.success() {
                return Err(eyre::eyre!(
                    "Borg backup failed: {}",
                    String::from_utf8_lossy(&borg_output.stderr)
                ));
            }

            info!("Borg backup completed successfully");

            Ok(())
        }
        .await
        .wrap_err("Failed to run borg backup");

        if let Err(e) = borg_results {
            return Err(e);
        }

        Ok(())
    }
}
