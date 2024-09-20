use std::sync::Arc;

use tracing::{debug, info};

use crate::{
    config::{AppConfig, JobConfig},
    storage::{self, StorageHandler},
    xapi::{cli::client::XApiCliClient, SnapshotType},
};

use super::{JobType, XenbakJob};

#[derive(Clone, Debug)]
pub struct VmBackupJob {
    pub job_type: JobType,
    pub app_config: AppConfig,
    pub job_config: JobConfig,
}

#[async_trait::async_trait]
impl XenbakJob for VmBackupJob {
    fn new(app_config: AppConfig, job_config: JobConfig) -> VmBackupJob {
        VmBackupJob {
            job_type: JobType::VmBackup,
            app_config,
            job_config,
        }
    }

    fn get_schedule(&self) -> String {
        self.job_config.schedule.clone()
    }

    /// runs a full vm backup job
    async fn run(&self) -> eyre::Result<()> {
        // let span = tracing::info_span!("job", job_name = %self.job_config.name);
        // let _guard = span.enter();

        info!("Running VM backup job with name: {}", self.job_config.name);
        // get filtered VM UUIDs
        let vms = XApiCliClient::filter_vms_by_tag(
            self.job_config.tag_filter.clone(),
            self.job_config.tag_filter_exclude.clone(),
        )
        .await?;

        debug!("Found {} VMs to backup", vms.len());

        let local_storages = self
            .app_config
            .storage
            .local
            .iter()
            .map(|config| storage::local::LocalStorage::new(config.clone()))
            .collect::<Vec<storage::local::LocalStorage>>();

        // sempahore to limit concurrency
        let permits = Arc::new(tokio::sync::Semaphore::new(
            self.job_config.concurrency as usize,
        ));

        // stores all thread/task handles
        let mut handles = vec![];

        // iterate over VM UUIDs and perform backup for each
        for vm in vms {
            // get a permit from the semaphore
            let permit = permits.clone().acquire_owned().await.unwrap();

            // clone the local storages and other required data for the task
            let local_storages = local_storages.clone();
            let job_type = self.job_type.clone();
            let job_config = self.job_config.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;

                tracing::info!("Backing up VM {} [{}]", vm.name_label, vm.uuid);

                let snapshot = XApiCliClient::snapshot(&vm, SnapshotType::Normal).await?;
                dbg!(&snapshot);

                let backup_result = async {
                    // create snapshot for the VM and set is-a-template to false
                    info!("Creating snapshot...");
                    debug!("Setting is-a-template to false...");
                    let mut snapshot =
                        XApiCliClient::set_snapshot_param_not_template(&snapshot).await?;
                    dbg!(&snapshot);

                    let backup_object = storage::BackupObject::new(
                        job_type.clone(),
                        vm.name_label.clone(),
                        snapshot.snapshot_time,
                        job_config.compression.clone(),
                    );

                    snapshot = XApiCliClient::set_snapshot_name(
                        &snapshot,
                        backup_object.generate_name_without_extension().as_ref(),
                    )
                    .await?;

                    dbg!(&snapshot);

                    for ls in local_storages {
                        let full_path = ls.generate_full_path(backup_object.clone());

                        // exporting vm to file
                        debug!("Exporting VM to file: {}", full_path);
                        XApiCliClient::vm_export_to_file(
                            &snapshot.uuid,
                            full_path.as_ref(),
                            job_config.compression.clone(),
                        )
                        .await?;

                        debug!("Rotating backups in {}", ls.config.path);
                        let backup_object_filter =
                            storage::BackupObjectFilter::from_backup_object(backup_object.clone());
                        ls.rotate(backup_object_filter, job_config.retention)
                            .await?;
                    }

                    Ok::<(), eyre::Error>(())
                }
                .await;

                // always delete the snapshot
                debug!("Deleting snapshot...");
                XApiCliClient::delete_snapshot_by_uuid(&snapshot.uuid).await?;

                backup_result?;

                info!("Finished backing up VM {}", vm.name_label);

                // drop the permit to release it back to the semaphore
                drop(_permit);

                Ok::<(), eyre::Error>(())
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await??;
        }

        info!("Finished VM backup job with name: {}", self.job_config.name);

        Ok(())
    }
}
