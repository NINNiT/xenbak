use std::sync::Arc;

use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

use crate::{jobs::XenbakJob, monitoring::MonitoringTrait, GlobalState};

pub struct XenbakScheduler {
    scheduler: JobScheduler,
}

impl XenbakScheduler {
    pub async fn new() -> XenbakScheduler {
        XenbakScheduler {
            scheduler: JobScheduler::new().await.unwrap(),
        }
    }

    async fn execute_job_with_monitoring<X: XenbakJob + Send + Clone + Sync + 'static>(
        job: &mut X,
        global_state: Arc<GlobalState>,
    ) {
        let mut monitoring_services: Vec<Arc<dyn MonitoringTrait>> = vec![];
        if let Some(mail_service) = global_state.mail_service.clone() {
            monitoring_services.push(Arc::new(mail_service) as Arc<dyn MonitoringTrait>);
        }
        if let Some(healthchecks_service) = global_state.healthchecks_service.clone() {
            monitoring_services
                .push(Arc::new(healthchecks_service).clone() as Arc<dyn MonitoringTrait>);
        }

        for service in &monitoring_services {
            service.start(job.get_name()).await.unwrap();
        }

        // run the job
        let job_result = job.run().await;
        let job_stats = job.get_job_stats();

        // send success/failure notification
        if let Err(e) = job_result {
            error!("{:?}", e);
            for service in &monitoring_services {
                service
                    .failure(job_stats.config.name.clone(), job_stats.clone())
                    .await
                    .unwrap();
            }
        } else {
            for service in &monitoring_services {
                service
                    .success(job_stats.config.name.clone(), job_stats.clone())
                    .await
                    .unwrap();
            }
        }
    }

    pub async fn add_job<X: XenbakJob + Send + Clone + Sync + 'static>(
        &mut self,
        job: X,
        global_state: Arc<GlobalState>,
    ) -> eyre::Result<()> {
        let span = tracing::span!(tracing::Level::DEBUG, "XenbakScheduler::add_job");
        let _enter = span.enter();
        info!(
            "Adding job '{}' [{}] to scheduler",
            job.get_name(),
            job.get_schedule()
        );
        self.scheduler
            .add(Job::new_async(
                job.get_schedule().as_ref(),
                move |mut _uuid, mut _l| {
                    let mut job = job.clone();
                    let global_state = global_state.clone();
                    Box::pin(async move {
                        Self::execute_job_with_monitoring(&mut job, global_state).await;
                    })
                },
            )?)
            .await
            .unwrap();
        Ok(())
    }

    pub async fn run_once<X: XenbakJob + Send + Clone + Sync + 'static>(
        &mut self,
        job: X,
        global_state: Arc<GlobalState>,
    ) -> eyre::Result<()> {
        let span = tracing::span!(tracing::Level::DEBUG, "XenbakScheduler::run_once");
        let _enter = span.enter();
        info!("Running job '{}' once", job.get_name());
        Self::execute_job_with_monitoring(&mut job.clone(), global_state).await;
        Ok(())
    }

    pub async fn start(&mut self) {
        self.scheduler.start().await.unwrap();
    }
}
