use std::str::FromStr;

use crate::config::{AppConfig, JobConfig};

pub mod vm_backup;

#[derive(Debug, Clone, PartialEq)]
pub enum JobType {
    VmBackup,
}

impl ToString for JobType {
    fn to_string(&self) -> String {
        match self {
            JobType::VmBackup => "vm".to_string(),
        }
    }
}

impl FromStr for JobType {
    type Err = eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "vm" => Ok(JobType::VmBackup),
            _ => Err(eyre::eyre!("Invalid job type")),
        }
    }
}

#[async_trait::async_trait]
pub trait XenbakJob {
    fn new(app_config: AppConfig, job_config: JobConfig) -> Self;
    fn get_schedule(&self) -> String;
    async fn run(&self) -> eyre::Result<()>;
}
