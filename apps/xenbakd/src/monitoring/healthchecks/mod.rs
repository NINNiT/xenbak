use std::collections::HashMap;

use eyre::ContextCompat;
use reqwest::header::HeaderMap;
use reqwest_middleware::ClientWithMiddleware;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

use tracing::debug;

mod types;

use crate::{
    config::{HealthchecksConfig, JobConfig},
    jobs::XenbakJobStats,
};

use self::types::{
    HealthchecksCheckInfo, HealthchecksCreateCheckRequest, HealthchecksListChecksResponse,
};

use super::MonitoringTrait;

#[derive(Clone, Debug)]
pub struct HealthchecksService {
    config: HealthchecksConfig,
    client: ClientWithMiddleware,
    checks: HashMap<String, HealthchecksCheckInfo>,
}

impl HealthchecksService {
    /// builds the service from a config
    pub fn from_config(config: HealthchecksConfig) -> Self {
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(config.max_retries);

        HealthchecksService {
            config,
            client: reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
                .with(RetryTransientMiddleware::new_with_policy(retry_policy))
                .build(),
            checks: HashMap::new(),
        }
    }

    /// generates 'X-Api-Key' header for healthchecks.io api requests
    async fn generate_auth_header(&self) -> eyre::Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Api-Key",
            self.config
                .api_key
                .as_str()
                .parse()
                .expect("Failed to parse api key"),
        );

        Ok(headers)
    }

    async fn generate_slug(&self, job_name: String, hostname: String) -> String {
        format!("{}_{}", job_name, hostname)
    }
}

#[async_trait::async_trait]
impl MonitoringTrait for HealthchecksService {
    async fn success(
        &self,
        hostname: String,
        job_name: String,
        job_stats: XenbakJobStats,
    ) -> eyre::Result<()> {
        debug!(
            "Sending success notification for job '{}' on host '{}'",
            job_name, hostname
        );

        let check = self
            .checks
            .get(&self.generate_slug(job_name, hostname).await)
            .context("Check not found")?;

        self.client
            .post(check.ping_url.clone())
            .json(&job_stats)
            .send()
            .await?;

        Ok(())
    }

    async fn start(&self, hostname: String, job_name: String) -> eyre::Result<()> {
        debug!(
            "Sending start notification for job '{}' on host '{}'",
            job_name, hostname
        );

        let check = self
            .checks
            .get(&self.generate_slug(job_name, hostname).await)
            .context("Check not found")?;

        self.client
            .post(format!("{}/{}", check.ping_url, "start"))
            .send()
            .await?;

        Ok(())
    }

    async fn failure(
        &self,
        hostname: String,
        job_name: String,
        job_stats: XenbakJobStats,
    ) -> eyre::Result<()> {
        debug!(
            "Sending failure notification for job '{}' on host '{}'",
            job_name, hostname
        );

        let check = self
            .checks
            .get(&self.generate_slug(job_name, hostname).await)
            .context("Check not found")?;

        self.client
            .post(format!("{}/{}", check.ping_url, "fail"))
            .json(&job_stats)
            .send()
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
pub trait HealthchecksManagementApiTrait {
    async fn list_checks(
        &self,
        tag_filter: Option<Vec<String>>,
        slug_filter: Option<String>,
    ) -> eyre::Result<HealthchecksListChecksResponse>;
    async fn initialize(&mut self, jobs: Vec<JobConfig>, hostname: String) -> eyre::Result<()>;
}

#[async_trait::async_trait]
impl HealthchecksManagementApiTrait for HealthchecksService {
    /// List all checks for the current healthchecks.io project
    async fn list_checks(
        &self,
        tag_filter: Option<Vec<String>>,
        slug_filter: Option<String>,
    ) -> eyre::Result<HealthchecksListChecksResponse> {
        let url = format!("{}/api/v3/checks", self.config.server);
        let mut request = self
            .client
            .get(&url)
            .headers(self.generate_auth_header().await?);

        if let Some(tag_filter) = tag_filter {
            for tag in tag_filter {
                request = request.query(&[("tag", tag)]);
            }
        }

        if let Some(slug_filter) = slug_filter {
            request = request.query(&[("slug", slug_filter)]);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            let checks: HealthchecksListChecksResponse = response.json().await?;
            Ok(checks)
        } else {
            Err(eyre::eyre!(
                "Failed to list healthchecks.io checks ({}): {}",
                response.status(),
                response.text().await?
            ))
        }
    }

    /// creates or updates healthchecks.io checks for each job
    /// - if a check already exists, it will be updated
    /// - if a check does not exist, it will be created
    async fn initialize(&mut self, jobs: Vec<JobConfig>, hostname: String) -> eyre::Result<()> {
        // iterate over configured jobs, update or create checks
        for job in jobs {
            let tags = vec![hostname.as_ref()].join(" ");
            let name = self.generate_slug(job.name.clone(), hostname.clone()).await;
            let slug = name.clone();
            let grace = self.config.grace;
            let schedule = job
                .schedule
                .split_whitespace()
                .skip(1)
                .collect::<Vec<&str>>()
                .join(" ");

            debug!(name);

            let create_url = format!("{}/api/v3/checks/", self.config.server);

            let request = HealthchecksCreateCheckRequest {
                name: name.clone(),
                tags,
                schedule,
                grace,
                timeout: 86400,
                slug,
                unique: vec!["name".into()],
            };

            let response: HealthchecksCheckInfo = self
                .client
                .post(&create_url)
                .headers(self.generate_auth_header().await?)
                .json(&request)
                .send()
                .await?
                .json()
                .await?;

            self.checks.insert(name.clone(), response);
        }

        Ok(())
    }
}
