use crate::jobs::XenbakJobStats;

pub mod healthchecks;
pub mod mail;

#[async_trait::async_trait]
pub trait MonitoringTrait: Send + Sync {
    async fn success(&self, job_name: String, job_stats: XenbakJobStats) -> eyre::Result<()>;
    async fn failure(&self, job_name: String, job_stats: XenbakJobStats) -> eyre::Result<()>;
    async fn start(&self, job_name: String) -> eyre::Result<()>;
}
