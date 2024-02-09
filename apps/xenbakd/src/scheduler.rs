use tokio_cron_scheduler::{Job, JobScheduler};

use crate::jobs::{XenbakJob};

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
    ) -> eyre::Result<()> {
        self.scheduler
            .add(Job::new_async(
                job.get_schedule().as_ref(),
                move |mut _uuid, mut _l| {
                    let job = job.clone();
                    Box::pin(async move {
                        job.run().await.unwrap();
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

    pub async fn shutdown(&mut self) {
        self.scheduler.shutdown().await.unwrap();
    }
}
