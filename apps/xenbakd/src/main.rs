use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use jobs::{vm_backup::VmBackupJob, XenbakJob};
use tracing::Level;

use crate::{config::AppConfig, scheduler::XenbakScheduler};

mod config;
mod jobs;
mod monitoring;
mod scheduler;
mod storage;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // initialize colored eyre for better-looking error messages
    color_eyre::install().unwrap();

    // load default config, then override/merge using config.toml
    let config = Figment::from(Serialized::defaults(AppConfig::default()))
        .merge(Toml::file("config.toml"))
        .extract::<AppConfig>()
        .expect("Failed to load configuration");

    // initialize tracing/logging
    let log_level = match config.general.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_max_level(log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // creating job scheduler and adding jobs
    let mut scheduler = XenbakScheduler::new().await;
    for job in config.jobs.clone() {
        let backup_job = VmBackupJob::new(config.clone(), job.clone());
        scheduler.add_job(backup_job).await?;
    }

    // start scheduler
    scheduler.start().await;
    tokio::signal::ctrl_c().await.unwrap();

    Ok(())
}
