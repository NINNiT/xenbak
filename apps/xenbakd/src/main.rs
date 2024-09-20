const BANNER: &str = r#"
__  _____ _ __ | |__   __ _| | ____| |
\ \/ / _ \ '_ \| '_ \ / _` | |/ / _` |
 >  <  __/ | | | |_) | (_| |   < (_| |
/_/\_\___|_| |_|_.__/ \__,_|_|\_\__,_|
  "#;

mod cli;
mod config;
mod jobs;
mod monitoring;
mod scheduler;
mod storage;
mod xapi;

use crate::{
    config::AppConfig,
    jobs::{vm_backup::VmBackupJob, XenbakJob},
    monitoring::healthchecks::HealthchecksManagementApiTrait,
    scheduler::XenbakScheduler,
};
use clap::Parser;
use colored::Colorize;
use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use std::sync::Arc;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // initialize colored eyre for better-looking panics
    color_eyre::install().unwrap();

    // print banner
    println!("{}", BANNER.cyan());

    // parse cli args
    let cli = cli::XenbakdCli::parse();
    let config_path = cli.config;
    // load default config, then override/merge using config.toml
    let mut config = Figment::from(Serialized::defaults(AppConfig::default()))
        .merge(Toml::file(config_path))
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
        .with_ansi(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_max_level(log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("Starting Xenbakd!");

    // initialize healthchecks_service
    info!("Initializing healthchecks.io service...");
    let healthchecks_service: Option<monitoring::healthchecks::HealthchecksService> =
        match config.monitoring.healthchecks.enabled {
            true => {
                let mut service = monitoring::healthchecks::HealthchecksService::from_config(
                    config.monitoring.healthchecks.clone(),
                );

                match service.initialize(config.jobs.clone()).await {
                    Ok(_) => {
                        tracing::info!("Healthchecks service initialized successfully");
                        Some(service)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to initialize healthchecks service: {}", e);
                        tracing::warn!("Disabling healthchecks service...");
                        config.monitoring.healthchecks.enabled = false;
                        None
                    }
                }
            }
            false => {
                tracing::warn!("Healthchecks service is disabled");
                None
            }
        };

    // initialize mail_service
    info!("Initializing mail service...");
    let mail_service: Option<monitoring::mail::MailService> = match config.monitoring.mail.enabled {
        true => {
            let service =
                monitoring::mail::MailService::from_config(config.monitoring.mail.clone()).await;

            match service {
                Ok(service) => {
                    tracing::info!("Mail service initialized successfully");
                    Some(service)
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize mail service: {}", e);
                    tracing::warn!("Disabling mail service...");
                    config.monitoring.mail.enabled = false;
                    None
                }
            }
        }
        false => {
            tracing::warn!("Mail service is disabled");
            None
        }
    };

    // create global state
    let global_state = Arc::new(GlobalState {
        config: config.clone(),
        mail_service,
        healthchecks_service,
    });

    // match clap cli
    match cli.subcmd {
        cli::SubCommand::Daemon(_) => {
            let mut scheduler = XenbakScheduler::new().await;
            for job in config.jobs.clone() {
                if !job.enabled {
                    continue;
                }
                let backup_job = VmBackupJob::new(global_state.clone(), job.clone());
                scheduler.add_job(backup_job, global_state.clone()).await?;
            }
            // start scheduler
            scheduler.start().await;
            tokio::signal::ctrl_c().await.unwrap();
        }
        cli::SubCommand::Run(run) => {
            let mut scheduler = XenbakScheduler::new().await;

            for job in run.jobs {
                let job = config
                    .jobs
                    .iter()
                    .find(|j| j.name == job)
                    .expect("Given Job not found in config");

                let backup_job = VmBackupJob::new(global_state.clone(), job.clone());
                scheduler.run_once(backup_job, global_state.clone()).await?;
            }
        }
    }

    tokio::signal::ctrl_c().await.unwrap();

    Ok(())
}

#[derive(Debug, Clone)]
pub struct GlobalState {
    pub config: AppConfig,
    pub mail_service: Option<monitoring::mail::MailService>,
    pub healthchecks_service: Option<monitoring::healthchecks::HealthchecksService>,
}
