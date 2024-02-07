use tokio::process::Command as AsyncCommand;

use crate::error::XApiCliError;
use crate::types::vm::VM;
use crate::types::{clean_stdout, FromCliOutput, Uuid, Uuids};

pub enum SnapshotType {
    Normal,
    Memory,
}

impl ToString for SnapshotType {
    fn to_string(&self) -> String {
        match self {
            SnapshotType::Normal => "basic".to_string(),
            SnapshotType::Memory => "memory".to_string(),
        }
    }
}

pub struct XApiCliClient {}

impl XApiCliClient {
    /// filter by tags and return Uuids
    pub async fn filter_vms_by_tag(
        tags: Vec<String>,
        excluded_tags: Vec<String>,
    ) -> Result<Uuids, XApiCliError> {
        // get VM UUIDs with the specified tags
        let tagged_uuids: Vec<String>;
        let tagged_uuid_output = AsyncCommand::new("xe")
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
            tagged_uuids = Uuids::from_cli_output(&stdout)?;
        } else {
            let stderr = String::from_utf8_lossy(&tagged_uuid_output.stderr);
            return Err(XApiCliError::CommandFailed(stderr.into()));
        }

        // get VM UUIDs with the excluded tags
        let excluded_uuids: Vec<String>;
        let excluded_uuid_output = AsyncCommand::new("xe")
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
            excluded_uuids = Uuids::from_cli_output(&stdout)?;
        } else {
            let stderr = String::from_utf8_lossy(&excluded_uuid_output.stderr);
            return Err(XApiCliError::CommandFailed(stderr.into()));
        }

        // filter out the excluded UUIDs
        let final_uuids: Vec<String> = tagged_uuids
            .into_iter()
            .filter(|uuid| !excluded_uuids.contains(uuid))
            .collect();

        Ok(final_uuids)
    }

    pub async fn snapshot(
        vm_uuid: &str,
        snapshot_type: SnapshotType,
    ) -> Result<Uuid, XApiCliError> {
        let mut command = AsyncCommand::new("xe");

        // snapshot name with timestamp and type
        let timestamp = chrono::Utc::now().format("%Y-%m-%d-%H-%M-%S").to_string();
        let name = format!(
            "xenbakd-snapshot-{}-{}",
            snapshot_type.to_string(),
            timestamp
        );

        match snapshot_type {
            SnapshotType::Normal => {
                command
                    .arg("vm-snapshot")
                    .arg("new-name-label=".to_owned() + &name)
                    .arg("vm=".to_owned() + vm_uuid);
            }
            SnapshotType::Memory => {
                command
                    .arg("vm-checkpoint")
                    .arg("new-name-label=".to_owned() + &name)
                    .arg("vm=".to_owned() + vm_uuid);
            }
        }

        let output = command.output().await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Ok(Uuid::from_cli_output(&stdout)?);
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(XApiCliError::SnapshotFailure(stderr.into()));
        }
    }

    pub async fn delete_snapshot(snapshot_uuid: &str) -> Result<(), XApiCliError> {
        let output = AsyncCommand::new("xe")
            .arg("snapshot-uninstall")
            .arg("uuid=".to_owned() + snapshot_uuid)
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

    pub async fn vm_export_to_file(
        uuid: &str,
        filename: &str,
        compress: Option<String>,
    ) -> Result<(), XApiCliError> {
        let mut command = AsyncCommand::new("xe");

        command
            .arg("vm-export")
            .arg("filename=".to_owned() + filename)
            .arg("vm=".to_owned() + uuid);

        if let Some(compress) = compress {
            command.arg("compress=".to_owned() + &compress);
        }

        let output = command.output().await?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(XApiCliError::CommandFailed(stderr.into()));
        }
    }

    pub async fn dynamic_command(args: Vec<&str>) -> Result<String, XApiCliError> {
        let output = AsyncCommand::new("xe").args(args).output().await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(clean_stdout(&stdout))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(XApiCliError::CommandFailed(stderr.into()));
        }
    }

    pub async fn set_snapshot_param_not_template(snapshot_uuid: &str) -> Result<(), XApiCliError> {
        let output = AsyncCommand::new("xe")
            .arg("snapshot-param-set")
            .arg("is-a-template=false")
            .arg("uuid=".to_owned() + snapshot_uuid)
            .output()
            .await?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(XApiCliError::CommandFailed(stderr.into()));
        }
    }

    pub async fn get_vm_info(vm_uuid: &str) -> Result<VM, XApiCliError> {
        let output = AsyncCommand::new("xe")
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
