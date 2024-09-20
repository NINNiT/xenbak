#![allow(dead_code)]

use std::os::unix::{ffi::OsStringExt, fs::MetadataExt};

use crate::config::{JobConfig, LocalStorageConfig};

use super::{BackupObject, BackupObjectFilter, StorageHandler, StorageStatus, StorageType};

#[derive(Debug, Clone)]
pub struct LocalStorage {
    pub storage_type: StorageType,
    pub config: LocalStorageConfig,
}

impl LocalStorage {
    pub fn new(config: LocalStorageConfig) -> Self {
        LocalStorage {
            storage_type: StorageType::Local,
            config,
        }
    }
}

impl LocalStorage {
    pub fn generate_full_path(&self, backup_object: crate::storage::BackupObject) -> String {
        format!(
            "{}/{}",
            self.config.path,
            backup_object.generate_name_with_extension()
        )
    }
}

#[async_trait::async_trait]
impl StorageHandler for LocalStorage {
    async fn status(&self) -> eyre::Result<StorageStatus> {
        todo!()
    }

    fn storage_type(&self) -> StorageType {
        self.storage_type.clone()
    }

    async fn list(
        &self,
        filter: BackupObjectFilter,
    ) -> eyre::Result<Vec<crate::storage::BackupObject>> {
        let mut paths = tokio::fs::read_dir(&self.config.path).await?;
        let mut backup_objects: Vec<BackupObject> = vec![];

        while let Some(entry) = paths.next_entry().await? {
            let metadata = entry.metadata().await?;

            if metadata.is_file() {
                let file_name = entry.file_name().into_string().map_err(|os_string| {
                    eyre::eyre!("Failed to convert OsString to String: {:?}", os_string)
                })?;

                let backup_object = BackupObject::from_name_with_extension(file_name).await?;

                // apply filter
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

                // time_stamp: Option<(
                //      Option<chrono::DateTime<chrono::Utc>>,
                //      Option<chrono::DateTime<chrono::Utc>>,
                //  )>,
                if let Some(time_stamp) = filter.time_stamp.clone() {
                    if let Some(start) = time_stamp.0 {
                        if let Some(end) = time_stamp.1 {
                            if start <= backup_object.time_stamp && backup_object.time_stamp <= end
                            {
                                continue;
                            }
                        } else {
                            if start <= backup_object.time_stamp {
                                continue;
                            }
                        }
                    } else {
                        if let Some(end) = time_stamp.1 {
                            if backup_object.time_stamp <= end {
                                continue;
                            }
                        }
                    }
                }

                if let Some(compression) = filter.compression.clone() {
                    if let Some(backup_object_compression) = backup_object.compression.clone() {
                        if !compression.contains(&backup_object_compression) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }

                backup_objects.push(backup_object);
            }
        }

        Ok(backup_objects)
    }

    async fn rotate(&self, filter: BackupObjectFilter, retention: u32) -> eyre::Result<()> {
        dbg!(&filter);
        let backup_objects = self.list(filter).await?;
        dbg!(&backup_objects);

        let mut vm_job_type_map: std::collections::HashMap<String, Vec<BackupObject>> =
            std::collections::HashMap::new();

        for backup_object in backup_objects {
            let key = format!(
                "{}__{}",
                backup_object.job_type.to_string(),
                backup_object.vm_name
            );

            if let Some(backup_objects) = vm_job_type_map.get_mut(&key) {
                backup_objects.push(backup_object);
            } else {
                vm_job_type_map.insert(key, vec![backup_object]);
            }
        }

        dbg!(&vm_job_type_map);

        for (_key, mut backup_objects) in vm_job_type_map {
            backup_objects.sort_by(|a, b| b.time_stamp.cmp(&a.time_stamp));

            if backup_objects.len() > retention as usize {
                let to_delete = &backup_objects[retention as usize..];

                for backup_object in to_delete {
                    let full_path = self.generate_full_path(backup_object.clone());
                    dbg!(&full_path);
                    tokio::fs::remove_file(full_path).await?;
                }
            }
        }

        Ok(())
    }
}
