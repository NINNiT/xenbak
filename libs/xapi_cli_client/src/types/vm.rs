use std::str::FromStr;

use crate::error::XApiCliError;

use super::{parse_timestamp, FromCliOutput};

#[derive(Debug, Default)]
pub struct VM {
    pub uuid: String,
    pub name_label: String,
    pub name_description: String,
    pub is_a_template: bool,
    pub is_default_template: bool,
    pub is_a_snapshot: bool,
    pub snapshot_time: chrono::DateTime<chrono::Utc>,
}

impl FromCliOutput for VM {
    /// create a new VM struct from `xe vm-param-list` stdout
    fn from_cli_output(output: &str) -> Result<VM, XApiCliError> {
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
