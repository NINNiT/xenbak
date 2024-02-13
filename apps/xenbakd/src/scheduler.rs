use std::sync::Arc;

use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

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
    pub async fn add_job<X: XenbakJob + Send + Clone + Sync + 'static>(
        &mut self,
        job: X,
        global_state: Arc<GlobalState>,
    ) -> eyre::Result<()> {
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
                        let mut monitoring_services: Vec<Arc<dyn MonitoringTrait>> = vec![];
                        if let Some(mail_service) = global_state.mail_service.clone() {
                            monitoring_services
                                .push(Arc::new(mail_service) as Arc<dyn MonitoringTrait>);
                        }
                        if let Some(healthchecks_service) =
                            global_state.healthchecks_service.clone()
                        {
                            monitoring_services
                                .push(Arc::new(healthchecks_service).clone()
                                    as Arc<dyn MonitoringTrait>);
                        }

                        for service in &monitoring_services {
                            service
                                .start(global_state.config.general.hostname.clone(), job.get_name())
                                .await
                                .unwrap();
                        }

                        // run the joby
                        let job_result = job.run().await;
                        let job_stats = job.get_job_stats();

                        // send success/failure notification
                        if let Err(_e) = job_result {
                            for service in &monitoring_services {
                                service
                                    .failure(
                                        job_stats.hostname.clone(),
                                        job_stats.job_name.clone(),
                                        job_stats.clone(),
                                    )
                                    .await
                                    .unwrap();
                            }
                        } else {
                            for service in &monitoring_services {
                                service
                                    .success(
                                        job_stats.hostname.clone(),
                                        job_stats.job_name.clone(),
                                        job_stats.clone(),
                                    )
                                    .await
                                    .unwrap();
                            }
                        }
                    })
                },
            )?)
            .await
            .unwrap();
        Ok(())
    }

    pub async fn start(&mut self) {
        self.scheduler.start().await.unwrap();
    }
}
