use std::sync::Arc;

use tracing::{debug, info};
use xapi_cli_client::client::{SnapshotType, XApiCliClient};

use crate::config::{AppConfig, JobConfig};

use super::XenbakJob;

#[derive(Clone, Debug)]
pub struct VmBackupJob {
    pub app_config: AppConfig,
    pub job_config: JobConfig,
}

#[async_trait::async_trait]
impl XenbakJob for VmBackupJob {
    fn new(app_config: AppConfig, job_config: JobConfig) -> VmBackupJob {
        VmBackupJob {
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
        let uuids = XApiCliClient::filter_vms_by_tag(
            self.job_config.tag_filter.clone(),
            self.job_config.tag_filter_exclude.clone(),
        )
        .await?;

        debug!("Found {} VMs to backup", uuids.len());

        // sempahore to limit concurrency
        let permits = Arc::new(tokio::sync::Semaphore::new(
            self.job_config.concurrency as usize,
        ));

        // stores all thread/task handles
        let mut handles = vec![];

        // iterate over VM UUIDs and perform backup for each
        for id in uuids {
            let permit = permits.clone().acquire_owned().await.unwrap();
            let handle = tokio::spawn(async move {
                let _permit = permit;

                // get information about the VM
                let vm = XApiCliClient::get_vm_info(&id).await?;
                //
                // let span = tracing::info_span!("vm_backup", vm.name_label = %vm.name_label);
                // let guard = span.enter();

                tracing::info!("Backing up VM {} [{}]", vm.name_label, vm.uuid);
                // create snapshot for the VM
                info!("Creating snapshot...");
                let snapshot_uuid = XApiCliClient::snapshot(&vm.uuid, SnapshotType::Normal).await?;

                // set is-a-template to false for the previously created snapshot
                debug!("Setting is-a-template to false...");
                XApiCliClient::set_snapshot_param_not_template(&snapshot_uuid).await?;

                // export the VM to disk
                info!("Exporting VM to disk...");

                // let backup_file_path = format!(
                //     "{}/{}.xva",
                //     app_config.storage.local.first().unwrap().path.clone(),
                //     snapshot_uuid
                // );
                // XApiCliClient::vm_export_to_file(&snapshot_uuid, &backup_file_path, None).await?;

                // delete the snapshot once everything is done
                info!("Deleting snapshot...");
                XApiCliClient::delete_snapshot(&snapshot_uuid).await?;

                info!("Finished backing up VM {}", vm.name_label);

                drop(_permit);

                Ok::<(), eyre::Error>(())
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.await??;
        }

        info!("Finished VM backup job with name: {}", self.job_config.name);

        // create a new job for each VM

        Ok(())
    }
}
