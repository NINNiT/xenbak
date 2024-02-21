use std::str::FromStr;
use std::sync::Arc;

use serde::Serialize;

use crate::config::JobConfig;
use crate::GlobalState;

pub mod vm_backup;

#[async_trait::async_trait]
pub trait XenbakJob {
    fn new(global_state: Arc<GlobalState>, job_config: JobConfig) -> Self;
    fn get_schedule(&self) -> String;
    fn get_name(&self) -> String;
    fn get_job_type(&self) -> JobType;
    fn get_job_stats(&self) -> XenbakJobStats;
    async fn run(&mut self) -> eyre::Result<()>;
}

#[derive(Debug, Clone, Serialize)]
pub struct XenbakJobStats {
    pub config: JobConfig,
    pub total_objects: u32,
    pub successful_objects: u32,
    pub failed_objects: u32,
    pub duration: f64,
    pub errors: Vec<String>,
}

impl Default for XenbakJobStats {
    fn default() -> XenbakJobStats {
        XenbakJobStats {
            config: JobConfig::default(),
            total_objects: 0,
            successful_objects: 0,
            failed_objects: 0,
            duration: 0.0,
            errors: vec![],
        }
    }
}

impl XenbakJobStats {}

#[derive(Debug, Clone, PartialEq, Serialize)]
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
