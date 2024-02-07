use std::{future::Future, pin::Pin};

use crate::config::{AppConfig, JobConfig};

pub mod vm_backup;

#[async_trait::async_trait]
pub trait XenbakJob {
    fn new(app_config: AppConfig, job_config: JobConfig) -> Self;
    fn get_schedule(&self) -> String;
    async fn run(&self) -> eyre::Result<()>;
}
