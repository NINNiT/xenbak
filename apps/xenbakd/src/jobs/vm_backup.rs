use std::sync::Arc;

use eyre::Context;
use tracing::{debug, error, info, warn};

use crate::{
    config::JobConfig,
    jobs::XenbakJobStats,
    storage::{self, StorageHandler},
    xapi::{cli::client::XApiCliClient, SnapshotType},
    GlobalState,
};

use super::{JobType, XenbakJob};

#[derive(Clone, Debug)]
pub struct VmBackupJob {
    pub job_type: JobType,
    pub job_config: JobConfig,
    pub job_stats: XenbakJobStats,
    pub global_state: Arc<GlobalState>,
}

#[async_trait::async_trait]
impl XenbakJob for VmBackupJob {
    fn new(global_state: Arc<GlobalState>, job_config: JobConfig) -> VmBackupJob {
        VmBackupJob {
            job_type: JobType::VmBackup,
            global_state,
            job_config,
            job_stats: XenbakJobStats::default(),
        }
    }

    fn get_name(&self) -> String {
        self.job_config.name.clone()
    }

    fn get_job_type(&self) -> JobType {
        self.job_type.clone()
    }

    fn get_schedule(&self) -> String {
        self.job_config.schedule.clone()
    }

    fn get_job_stats(&self) -> XenbakJobStats {
        self.job_stats.clone()
    }

    /// runs a full vm backup job
    async fn run(&mut self) -> eyre::Result<()> {
        let timer = tokio::time::Instant::now();
        info!("Running VM backup job '{}'", self.job_config.name);

        self.job_stats.job_name = self.job_config.name.clone();
        self.job_stats.job_type = self.job_type.clone();
        self.job_stats.hostname = self.global_state.config.general.hostname.clone();
        self.job_stats.schedule = self.job_config.schedule.clone();

        // filter VMs by tag and tag_exclude
        let vms = XApiCliClient::filter_vms_by_tag(
            self.job_config.tag_filter.clone(),
            self.job_config.tag_filter_exclude.clone(),
        )
        .await?;
        debug!("{} objects affected by backup job", vms.len());
        self.job_stats.total_objects = vms.len() as u32;

        // if no VMs are found, return early
        if vms.is_empty() {
            warn!("No VMs found for backup job '{}'", self.job_config.name);
        }

        // create local storage instances from config
        let local_storages = self
            .global_state
            .config
            .storage
            .local
            .iter()
            .map(|config| {
                storage::local::LocalStorage::new(config.clone(), self.job_config.clone())
            })
            .collect::<Vec<storage::local::LocalStorage>>();

        // initialize all local storages (creating missing dirs, etc.)
        for ls in local_storages.iter() {
            ls.initialize().await?;
        }

        // sempahore to limit concurrency, use arc to share across threads
        let permits = Arc::new(tokio::sync::Semaphore::new(
            self.job_config.concurrency as usize,
        ));

        // this will store all thread/task handles
        let mut handles = vec![];

        // iterate over previously filtered VMs and perform backup for each
        for vm in vms {
            // get a permit from the semaphore
            let permit = permits.clone().acquire_owned().await.unwrap();

            // clone required data for the task, to move into the task
            let local_storages = local_storages.clone();
            let job_type = self.job_type.clone();
            let job_config = self.job_config.clone();

            // the backup task itself
            let handle = tokio::spawn(async move {
                let _permit = permit;

                let timer = tokio::time::Instant::now();

                info!("Starting backup of VM '{}' [{}]", vm.name_label, vm.uuid);

                // perform snapshot
                debug!("Creating snapshot...");
                let snapshot = XApiCliClient::snapshot(&vm, SnapshotType::Normal).await?;

                let backup_result = async {
                    // set is-a-template to false
                    debug!("Setting is-a-template to false...");
                    let mut snapshot =
                        XApiCliClient::set_snapshot_param_not_template(&snapshot).await?;

                    // create backup object
                    let backup_object = storage::BackupObject::new(
                        job_type.clone(),
                        vm.name_label.clone(),
                        snapshot.snapshot_time,
                        job_config.compression.clone(),
                    );

                    // set snapshot name to backup object name
                    snapshot = XApiCliClient::set_snapshot_name(
                        &snapshot,
                        backup_object.generate_name_without_extension().as_ref(),
                    )
                    .await?;

                    // iterate through enabled local storages, export VM for each and rotate backups
                    for ls in local_storages {
                        let full_path = ls.generate_full_file_path(backup_object.clone());

                        // exporting vm to file
                        debug!("Exporting VM to file: {}", full_path);
                        XApiCliClient::vm_export_to_file(
                            &snapshot.uuid,
                            full_path.as_ref(),
                            job_config.compression.clone(),
                        )
                        .await?;

                        // perform backup rotation on all files where filter matches
                        debug!("Rotating backups in {}", ls.storage_config.path);
                        let backup_object_filter =
                            storage::BackupObjectFilter::from_backup_object(backup_object.clone());
                        ls.rotate(backup_object_filter, job_config.retention)
                            .await?;
                    }

                    Ok::<(), eyre::Error>(())
                }
                .await;

                // always delete the snapshot...
                debug!("Deleting snapshot...");
                XApiCliClient::delete_snapshot_by_uuid(&snapshot.uuid).await?;

                // ...but propagate any errors that occurred during backup
                backup_result.wrap_err(format!(
                    "Failed to backup VM {} [{}]",
                    vm.name_label, vm.uuid
                ))?;

                let elapsed = timer.elapsed().as_secs_f64();

                info!(
                    "Finished backup of VM '{}' [{}] in {} seconds",
                    vm.name_label, vm.uuid, elapsed
                );

                // drop the permit to allow another task to run
                drop(_permit);

                Ok::<(), eyre::Error>(())
            });

            handles.push(handle);
        }

        // wait for all async/threaded tasks to finish, save the results
        let mut results = vec![];
        for handle in handles {
            results.push(handle.await);
        }

        // check if there are any errors in the results
        for result in results.iter() {
            if let Err(e) = result {
                error!("Error during backup job: {:?}", e);

                self.job_stats.failed_objects += 1;
                self.job_stats.errors.push(e.to_string());
            } else {
                self.job_stats.successful_objects += 1;
            }
        }

        let elapsed = timer.elapsed();
        self.job_stats.duration = elapsed.as_secs_f64();

        info!(
            "Finished VM backup job with name '{}' in {} seconds",
            self.job_config.name, self.job_stats.duration
        );

        if results.iter().any(|r| r.is_err()) {
            Err(eyre::eyre!("Backup job failed"))
        } else {
            Ok(())
        }
    }
}
