use super::{
    error::{XApiCliError, XApiParseError},
    parse_timestamp, UUIDs, UUID, VM,
};
use std::str::FromStr;

pub mod client;

pub trait FromCliOutput: Sized {
    fn from_cli_output(output: &str) -> Result<Self, XApiCliError>;
}

impl FromCliOutput for VM {
    /// create a new VM struct from `xe vm-param-list` stdout
    fn from_cli_output(output: &str) -> Result<VM, XApiCliError> {
        let output = output.trim();
        let mut vm = VM::default();

        for line in output.lines() {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }
            let key = parts[0].trim().split(' ').next().unwrap();
            let value = parts[1].trim();

            match key {
                "uuid" => vm.uuid = value.to_string(),
                "name-label" => vm.name_label = value.to_string(),
                "name-description" => vm.name_description = value.to_string(),
                "is-a-template" => vm.is_a_template = bool::from_str(value).unwrap(),
                "is-default-template" => vm.is_default_template = bool::from_str(value).unwrap(),
                "is-a-snapshot" => vm.is_a_snapshot = bool::from_str(value).unwrap(),
                "snapshot-time" => {
                    vm.snapshot_time = parse_timestamp(value)?;
                }
                _ => {}
            }
        }

        Ok(vm)
    }
}

impl FromCliOutput for UUID {
    fn from_cli_output(output: &str) -> Result<UUID, XApiCliError> {
        let output = output.replace("\n", "").trim().to_string();
        Ok(output)
    }
}
impl FromCliOutput for UUIDs {
    fn from_cli_output(output: &str) -> Result<UUIDs, XApiCliError> {
        let output = output.replace("\n", "").trim().to_string();

        let uuids: Vec<UUID> = output
            .split(',')
            .map(|uuid| uuid.trim().to_string())
            .collect();

        Ok(uuids)
    }
}
