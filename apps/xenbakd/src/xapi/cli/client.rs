use std::{process::Stdio, sync::Arc};

use tokio::process::Command as AsyncCommand;

use crate::{
    config::XenConfig,
    storage::{CompressionType, StorageHandler},
    xapi::{error::XApiCliError, SnapshotType, UUIDs, UUID, VM},
};

use super::FromCliOutput;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct XApiCliClient {
    config: XenConfig,
}

impl XApiCliClient {
    pub fn new(config: XenConfig) -> Self {
        XApiCliClient { config }
    }

    pub fn get_config(&self) -> &XenConfig {
        &self.config
    }

    pub fn get_base_command(&self) -> AsyncCommand {
        let mut command = AsyncCommand::new("xe");

        if self.config.server == "localhost" || self.config.server == "127.0.0.1" {
            command.arg("-s").arg("127.0.0.1");
        } else {
            command
                .arg("-s")
                .arg(&self.config.server)
                .arg("-u")
                .arg(&self.config.username)
                .arg("-pw")
                .arg(&self.config.password);
        }

        command
    }

    /// filter by tags and return UUIDs
    pub async fn filter_vms_by_tag(
        &self,
        tags: Vec<String>,
        excluded_tags: Vec<String>,
    ) -> Result<Vec<VM>, XApiCliError> {
        // get VM UUIDs with the specified tags
        let tagged_uuids: Vec<String>;
        let tagged_uuid_output = self
            .get_base_command()
            .arg("vm-list")
            .arg("tags:contains=".to_owned() + &tags.join(","))
            .arg("is-a-template=false")
            .arg("is-a-snapshot=false")
            .arg("is-control-domain=false")
            .arg("--minimal")
            .output()
            .await?;

        if tagged_uuid_output.status.success() {
            let stdout = String::from_utf8_lossy(&tagged_uuid_output.stdout);
            tagged_uuids = UUIDs::from_cli_output(&stdout)?;
        } else {
            let stderr = String::from_utf8_lossy(&tagged_uuid_output.stderr);
            return Err(XApiCliError::CommandFailed(stderr.into()));
        }

        // get VM UUIDs with the excluded tags
        let excluded_uuids: Vec<String>;
        let excluded_uuid_output = self
            .get_base_command()
            .arg("vm-list")
            .arg("is-a-template=false")
            .arg("is-a-snapshot=false")
            .arg("is-control-domain=false")
            .arg("tags:contains=".to_owned() + &excluded_tags.join(","))
            .arg("--minimal")
            .output()
            .await?;

        if excluded_uuid_output.status.success() {
            let stdout = String::from_utf8_lossy(&excluded_uuid_output.stdout);
            excluded_uuids = UUIDs::from_cli_output(&stdout)?;
        } else {
            let stderr = String::from_utf8_lossy(&excluded_uuid_output.stderr);
            return Err(XApiCliError::CommandFailed(stderr.into()));
        }

        // filter out the excluded UUIDs
        let final_uuids: UUIDs = tagged_uuids
            .into_iter()
            .filter(|uuid| !excluded_uuids.contains(uuid))
            .collect();

        let mut vms: Vec<VM> = vec![];

        for uuid in final_uuids {
            let vm = self.get_vm_by_uuid(&uuid).await?;
            vms.push(vm);
        }

        Ok(vms)
    }

    pub async fn snapshot(&self, vm: &VM, snapshot_type: SnapshotType) -> Result<VM, XApiCliError> {
        let mut command = self.get_base_command();

        match snapshot_type {
            SnapshotType::Normal => {
                command
                    .arg("vm-snapshot")
                    .arg("vm=".to_owned() + &vm.uuid)
                    .arg("new-name-label=xenbakd-snapshot");
            }
            SnapshotType::Memory => {
                command
                    .arg("vm-checkpoint")
                    .arg("vm=".to_owned() + &vm.uuid)
                    .arg("new-name-label=xenbakd-snapshot");
            }
        }

        let output = command.output().await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let uuid = UUID::from_cli_output(&stdout)?;
            self.get_vm_by_uuid(&uuid).await
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(XApiCliError::SnapshotFailure(stderr.into()));
        }
    }

    pub async fn set_snapshot_name(&self, snapshot: &VM, name: &str) -> Result<VM, XApiCliError> {
        let output = self
            .get_base_command()
            .arg("snapshot-param-set")
            .arg("uuid=".to_owned() + &snapshot.uuid)
            .arg("name-label=".to_owned() + name)
            .output()
            .await?;

        if output.status.success() {
            Ok(self.get_vm_by_uuid(&snapshot.uuid).await?)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(XApiCliError::CommandFailed(stderr.into()));
        }
    }

    pub async fn delete_snapshot_by_uuid(&self, snapshot: &UUID) -> Result<(), XApiCliError> {
        let output = self
            .get_base_command()
            .arg("snapshot-uninstall")
            .arg("uuid=".to_owned() + &snapshot)
            .arg("force=true")
            .output()
            .await?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(XApiCliError::CommandFailed(stderr.into()));
        }
    }

    // xe vm-export uuid=<VM_UUID> filename= | ssh <other_server> xe vm-import filename=/dev/stdin
    pub async fn vm_export_to_storage(
        &self,
        vm: &VM,
        storage_handler: Arc<dyn StorageHandler>,
        backup_object: crate::storage::BackupObject,
    ) -> eyre::Result<()> {
        let mut command = self.get_base_command();

        command
            .arg("vm-export")
            .arg("vm=".to_owned() + &vm.uuid)
            .arg("filename=");

        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        storage_handler
            .handle_stdio_stream(backup_object, stdout, stderr)
            .await?;

        Ok(())
    }

    pub async fn vm_export_to_file(
        &self,
        vm: &VM,
        filename: &str,
        compress: Option<CompressionType>,
    ) -> Result<(), XApiCliError> {
        let mut command = self.get_base_command();

        command
            .arg("vm-export")
            .arg("filename=".to_owned() + filename)
            .arg("vm=".to_owned() + &vm.uuid);

        if let Some(compress) = compress {
            command.arg("compress=".to_owned() + &compress.to_cli_arg());
        }

        let output = command.output().await?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(XApiCliError::CommandFailed(stderr.into()));
        }
    }

    pub async fn dynamic_command(&self, args: Vec<&str>) -> Result<String, XApiCliError> {
        let output = self.get_base_command().args(args).output().await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.into())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(XApiCliError::CommandFailed(stderr.into()));
        }
    }

    pub async fn set_snapshot_param_not_template(&self, snapshot: &VM) -> Result<VM, XApiCliError> {
        let output = self
            .get_base_command()
            .arg("snapshot-param-set")
            .arg("is-a-template=false")
            .arg("uuid=".to_owned() + &snapshot.uuid)
            .output()
            .await?;

        if output.status.success() {
            Ok(self.get_vm_by_uuid(&snapshot.uuid).await?)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(XApiCliError::CommandFailed(stderr.into()));
        }
    }

    pub async fn get_vm_by_uuid(&self, vm_uuid: &str) -> Result<VM, XApiCliError> {
        let output = self
            .get_base_command()
            .arg("vm-param-list")
            .arg("uuid=".to_owned() + vm_uuid)
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let vm = VM::from_cli_output(&stdout)?;
            Ok(vm)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(XApiCliError::CommandFailed(stderr.into()));
        }
    }
}
