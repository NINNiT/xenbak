use std::{collections::HashMap, sync::Arc};

use tracing::{debug, error, info, warn, Instrument};

use crate::{
    config::JobConfig,
    jobs::XenbakJobStats,
    storage,
    xapi::{
        cli::client::XApiCliClient,
        error::{XApiCliError, XApiParseError},
        SnapshotType, VM,
    },
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
        let job_timer = tokio::time::Instant::now();

        info!("Running VM backup job '{}'", self.job_config.name);

        self.job_stats.config = self.job_config.clone();

        // iterate through the job's configured xen hosts and create a XAPI client for each
        let xapi_clients: Vec<XApiCliClient> = self
            .job_config
            .get_xen_configs(self.global_state.config.xen.clone())
            .iter()
            .map(|x| XApiCliClient::new(x.clone()))
            .collect();

        // filter VMs by tag and map them to their respective XAPI clients (-> xen hosts)
        let mut vms: HashMap<XApiCliClient, Vec<VM>> = HashMap::new();

        for client in xapi_clients {
            let filtered_vms = client
                .filter_vms_by_tag(
                    self.job_config.tag_filter.clone(),
                    self.job_config.tag_filter_exclude.clone(),
                )
                .await?;
            vms.insert(client, filtered_vms);
        }

        // here's the total number of objects affected by the backup job
        self.job_stats.total_objects = vms.values().flatten().count() as u32;
        debug!(
            "{} objects affected by backup job",
            self.job_stats.total_objects
        );

        // if no VMs are found, print a warning
        if self.job_stats.total_objects == 0 {
            warn!("No VMs found for backup job '{}'", self.job_config.name);
        }

        // get all of the job's storage handlers...
        let storage_handlers = self
            .job_config
            .get_storages(self.global_state.config.storage.clone());

        // ... and initialize them (create sub-directories, create borg repo, ...)
        for storage_handler in storage_handlers.clone() {
            debug!(
                "Initializing storage handler '{}'",
                storage_handler.get_job_config().name
            );
            storage_handler.initialize().await?;
        }

        // sempahore to limit concurrent tasks, use arc to share across threads.
        let permits = Arc::new(tokio::sync::Semaphore::new(
            self.job_config.concurrency as usize,
        ));

        // this will store all thread/task handles
        let mut handles = vec![];

        // iterate over  VMs and perform backup for each
        for (xapi_client, vms) in vms {
            for vm in vms {
                let span = tracing::span!(
                    tracing::Level::INFO,
                    "VmBackupJob::run::backup_vm",
                    vm.name_label = vm.name_label.clone(),
                    xen.host = xapi_client.get_config().name.clone()
                );

                // get a permit from the semaphore
                let permit = permits.clone().acquire_owned().await.unwrap();

                // we have to clone this data, as it will be moved into a potential separate thread
                let storage_handlers = storage_handlers.clone();
                let job_type = self.job_type.clone();
                let xapi_client = xapi_client.clone();
                let job_config = self.job_config.clone();

                // the backup task itself - will be spawned into a separate thread/task
                let handle = tokio::spawn(async move {
                    let _permit = permit;
                    let vm_timer = tokio::time::Instant::now();
                    info!("Starting backup of VM '{}' [{}]", vm.name_label, vm.uuid);

                    // check if xenbakd should try to create a backup from an already-existing
                    // snapshot - otherwise create a temporary new one
                    let mut is_xenbakd_snapshot = true;
                    let snapshot: VM = match job_config.use_existing_snapshot {
                        true => {
                            // get all existing snapshots for the given VM
                            let existing_snapshots = xapi_client.get_snapshots(&vm).await;

                            // no snapshots? damn. create a new one.
                            if existing_snapshots.as_ref().is_err_and(|e| {
                                matches!(
                                    e,
                                    XApiCliError::XApiParseError(XApiParseError::EmptyOutput)
                                )
                            }) {
                                debug!("No recent snapshot found, creating new one");
                                xapi_client.snapshot(&vm, SnapshotType::Normal).await?
                            } else {
                                let mut existing_snapshots = existing_snapshots?;
                                // sort existing snapshots by snapshot time and get the most recent
                                existing_snapshots.sort_by(|a, b| {
                                    a.snapshot_time
                                        .timestamp()
                                        .partial_cmp(&b.snapshot_time.timestamp())
                                        .unwrap()
                                });
                                let newest_snapshot = existing_snapshots.last().unwrap();

                                // calculate snapshot age
                                let now = chrono::Utc::now();
                                let age_limit =
                                    job_config.use_existing_snapshot_age.unwrap_or(3600);
                                let snapshot_age = now - newest_snapshot.snapshot_time;

                                // check if the snapshot is within age limit
                                if snapshot_age.num_seconds() < age_limit {
                                    is_xenbakd_snapshot = false;
                                    newest_snapshot.clone()
                                } else {
                                    debug!(
                                        "Newest existing snapshot is older than {} seconds",
                                        age_limit
                                    );
                                    debug!("Creating new snapshot");
                                    xapi_client.snapshot(&vm, SnapshotType::Normal).await?
                                }
                            }
                        }
                        false => {
                            debug!("Creating new snapshot");
                            xapi_client.snapshot(&vm, SnapshotType::Normal).await?
                        }
                    };

                    let backup_result = async {
                        // set is-a-template to false
                        debug!("Setting is-a-template to false...");
                        let mut snapshot = xapi_client
                            .set_snapshot_param_not_template(&snapshot)
                            .await?;

                        // set snapshot name to a more readable format
                        if is_xenbakd_snapshot {
                            snapshot = xapi_client
                                .set_snapshot_name(
                                    &snapshot,
                                    format!("{}__{}", vm.name_label, snapshot.snapshot_time)
                                        .as_str(),
                                )
                                .await?;
                        }

                        // iterate through enabled local storages, export snapshost for each storage and rotate/cleanup backups
                        for storage_handler in storage_handlers {
                            // create the backup object
                            let backup_object = storage::BackupObject::new(
                                job_type.clone(),
                                vm.name_label.clone(),
                                xapi_client.get_config().name.clone(),
                                snapshot.snapshot_time,
                                None,
                            );

                            // export the snaphhot using the current storage handler
                            info!("Exporting VM to storage handler...",);
                            xapi_client
                                .vm_export_to_storage(
                                    &snapshot,
                                    storage_handler.clone(),
                                    backup_object.clone(),
                                )
                                .await?;

                            // rotate backups
                            debug!("Rotating backups");
                            let backup_object_filter =
                                storage::BackupObjectFilter::from_backup_object(
                                    backup_object.clone(),
                                );
                            storage_handler.rotate(backup_object_filter).await?;
                        }

                        Ok::<(), eyre::Error>(())
                    }
                    .await;

                    if is_xenbakd_snapshot {
                        debug!("Deleting snapshot...");
                        xapi_client.delete_snapshot_by_uuid(&snapshot.uuid).await?;
                    }

                    // propagate any errors that occurred during backup
                    if let Err(e) = backup_result {
                        return Err(e.wrap_err(format!(
                            "Backup of VM '{}' [{}] failed",
                            vm.name_label, vm.uuid
                        )));
                    }

                    // get the elapsed time and log it
                    let elapsed = vm_timer.elapsed().as_secs_f64();
                    info!(
                        "Finished backup of VM '{}' [{}] in {} seconds",
                        vm.name_label, vm.uuid, elapsed
                    );

                    // drop the permit to allow another task to run
                    drop(_permit);

                    eyre::Result::<()>::Ok(())
                })
                .instrument(span);
                // push the task handle into the handles vector to await it later
                handles.push(handle);
            }
        }

        // wait for all async/threaded tasks to finish and save the results into a vector
        let mut results = vec![];
        for handle in handles {
            results.push(handle.await?);
        }

        // check if there are any errors in the results, fill stats object appropiately
        for result in results.iter() {
            match result {
                Ok(_) => {
                    self.job_stats.successful_objects += 1;
                }
                Err(e) => {
                    let full_err = e
                        .chain()
                        .map(|e| e.to_string())
                        .collect::<Vec<String>>()
                        .join("\n");

                    self.job_stats.failed_objects += 1;
                    self.job_stats.errors.push(full_err.clone());
                    error!("{:?}", e);
                }
            }
        }

        // get the elapsed time
        let elapsed = job_timer.elapsed();
        self.job_stats.duration = elapsed.as_secs_f64();

        // if there were any errors, return an error
        if self.job_stats.failed_objects > 0 {
            return Err(eyre::eyre!("Backup job failed.",));
        }

        info!(
            "Finished VM backup job with name '{}' in {} seconds",
            self.job_config.name, self.job_stats.duration
        );

        // heck yeah, success!
        Ok(())
    }
}
