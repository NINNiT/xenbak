use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::{
    config::{JobConfig, LocalStorageConfig},
    jobs::JobType,
};

use super::{
    BackupObject, BackupObjectFilter, CompressionType, StorageHandler, StorageStatus, StorageType,
};

#[derive(Debug, Clone)]
pub struct LocalStorage {
    pub path: String,
    pub storage_type: StorageType,
    pub storage_config: LocalStorageConfig,
    pub job_config: JobConfig,
}

impl LocalStorage {
    pub fn new(storage_config: LocalStorageConfig, job_config: JobConfig) -> Self {
        LocalStorage {
            path: format!("{}/{}", storage_config.path, job_config.name),
            storage_type: StorageType::Local,
            job_config,
            storage_config,
        }
    }

    pub fn file_name_to_backup_object(&self, file_name: String) -> crate::storage::BackupObject {
        let parts: Vec<&str> = file_name.split("__").collect();
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

    pub fn backup_object_to_file_name(
        &self,
        backup_object: crate::storage::BackupObject,
    ) -> String {
        let base_name = format!(
            "{}__{}__{}__{}",
            backup_object.xen_host,
            backup_object.job_type.to_string(),
            backup_object.vm_name,
            backup_object.time_stamp.to_rfc3339()
        );

        let base_extension = match backup_object.job_type {
            JobType::VmBackup => "xva",
        };

        if self.storage_config.compression.is_none() {
            return format!("{}.{}", base_name, base_extension);
        } else {
            return format!(
                "{}.{}.{}",
                base_name,
                base_extension,
                self.storage_config
                    .compression
                    .as_ref()
                    .unwrap()
                    .to_extension()
            );
        };
    }
}

#[async_trait::async_trait]
impl StorageHandler for LocalStorage {
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
        let path = format!("{}/{}", self.storage_config.path, self.job_config.name);
        tokio::fs::create_dir_all(&path).await?;
        Ok(())
    }

    async fn list(
        &self,
        filter: BackupObjectFilter,
    ) -> eyre::Result<Vec<crate::storage::BackupObject>> {
        let mut paths = tokio::fs::read_dir(&self.path).await?;
        let mut backup_objects: Vec<BackupObject> = vec![];

        while let Some(entry) = paths.next_entry().await? {
            let metadata = entry.metadata().await?;

            if metadata.is_file() {
                let file_name = entry.file_name().into_string().map_err(|os_string| {
                    eyre::eyre!("Failed to convert OsString to String: {:?}", os_string)
                })?;

                let parts: Vec<&str> = file_name.split("__").collect();
                if parts.len() != 4 {
                    return Err(eyre::eyre!("Invalid backup object name"));
                }

                let backup_object = self.file_name_to_backup_object(file_name);

                // apply filter
                if let Some(xen_host) = filter.xen_host.clone() {
                    if !xen_host.contains(&backup_object.xen_host) {
                        continue;
                    }
                }

                if let Some(job_type) = filter.job_type.clone() {
                    if !job_type.contains(&backup_object.job_type) {
                        continue;
                    }
                }

                if let Some(vm_name) = filter.vm_name.clone() {
                    if !vm_name.contains(&backup_object.vm_name) {
                        continue;
                    }
                }

                if let Some(time_stamp) = filter.time_stamp.clone() {
                    if let Some(start) = time_stamp.0 {
                        if let Some(end) = time_stamp.1 {
                            if !(start <= backup_object.time_stamp
                                && backup_object.time_stamp <= end)
                            {
                                continue;
                            }
                        } else {
                            if !(start <= backup_object.time_stamp) {
                                continue;
                            }
                        }
                    } else {
                        if let Some(end) = time_stamp.1 {
                            if !(backup_object.time_stamp <= end) {
                                continue;
                            }
                        }
                    }
                }

                backup_objects.push(backup_object);
            }
        }

        Ok(backup_objects)
    }

    async fn rotate(&self, filter: BackupObjectFilter) -> eyre::Result<()> {
        let backup_objects = self.list(filter).await?;

        let mut vm_job_type_map: std::collections::HashMap<String, Vec<BackupObject>> =
            std::collections::HashMap::new();

        for backup_object in backup_objects {
            let key = format!(
                "{}__{}__{}",
                backup_object.xen_host,
                backup_object.job_type.to_string(),
                backup_object.vm_name
            );

            if let Some(backup_objects) = vm_job_type_map.get_mut(&key) {
                backup_objects.push(backup_object);
            } else {
                vm_job_type_map.insert(key, vec![backup_object]);
            }
        }

        // keep the last N backups
        for (_key, mut backup_objects) in vm_job_type_map {
            backup_objects.sort_by(|a, b| b.time_stamp.cmp(&a.time_stamp));

            if backup_objects.len() > self.storage_config.retention as usize {
                let to_delete = &backup_objects[self.storage_config.retention as usize..];

                for backup_object in to_delete {
                    let full_path = format!(
                        "{}/{}",
                        self.path,
                        self.backup_object_to_file_name(backup_object.clone()),
                    );
                    tokio::fs::remove_file(full_path).await?;
                }
            }
        }

        Ok(())
    }

    // receives an file stream fro m the XAPI client and handles I/O
    async fn handle_stdio_stream(
        &self,
        backup_object: BackupObject,
        stdout_stream: tokio::process::ChildStdout,
        stderr_stream: tokio::process::ChildStderr,
    ) -> eyre::Result<()> {
        // get full path for the file and create a handle
        let full_path = format!(
            "{}/{}",
            self.path,
            self.backup_object_to_file_name(backup_object.clone())
        );

        let result = async {
            // create file and get file handle
            let mut file = tokio::fs::File::create(&full_path).await?;

            // create a buffered stream reader for smoother I/O
            const BUFFER_SIZE: usize = 1024 * 1024 * 10;
            let mut stdout_buffered =
                tokio::io::BufReader::with_capacity(BUFFER_SIZE, stdout_stream);
            let mut stderr_buffered = tokio::io::BufReader::new(stderr_stream);

            match self.storage_config.compression {
                Some(LocalCompressionType::Zstd) => {
                    let mut zstd = async_compression::tokio::write::ZstdEncoder::new(file);
                    tokio::io::copy(&mut stdout_buffered, &mut zstd).await?;
                }
                Some(LocalCompressionType::Gzip) => {
                    let mut gzip = async_compression::tokio::write::GzipEncoder::new(file);
                    tokio::io::copy(&mut stdout_buffered, &mut gzip).await?;
                }
                None => {
                    tokio::io::copy(&mut stdout_buffered, &mut file).await?;
                }
            }

            // check stderr for errors
            let mut stderr = Vec::new();
            stderr_buffered.read_to_end(&mut stderr).await?;
            if !stderr.is_empty() {
                let stderr = String::from_utf8_lossy(&stderr);
                return Err(eyre::eyre!(
                    "Error encountered in stderr output: {}",
                    stderr
                ));
            }

            Ok::<(), eyre::Error>(())
        }
        .await;

        if let Err(e) = result {
            tokio::fs::remove_file(full_path).await?;
            return Err(e.wrap_err("Failed to write to file"));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocalStorageRetention {
    pub daily: u32,
    pub weekly: u32,
    pub monthly: u32,
    pub yearly: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum LocalCompressionType {
    #[serde(rename = "gzip")]
    Gzip,
    #[serde(rename = "zstd")]
    Zstd,
}

impl CompressionType for LocalCompressionType {
    fn to_extension(&self) -> String {
        match self {
            LocalCompressionType::Gzip => "gz".to_string(),
            LocalCompressionType::Zstd => "zst".to_string(),
        }
    }

    fn from_extension(extension: &str) -> eyre::Result<LocalCompressionType> {
        match extension {
            "gz" => Ok(LocalCompressionType::Gzip),
            "zst" => Ok(LocalCompressionType::Zstd),
            _ => Err(eyre::eyre!("Invalid compression extension")),
        }
    }

    fn to_cli_arg(&self) -> String {
        match self {
            LocalCompressionType::Gzip => "gzip".to_string(),
            LocalCompressionType::Zstd => "zstd".to_string(),
        }
    }
}
