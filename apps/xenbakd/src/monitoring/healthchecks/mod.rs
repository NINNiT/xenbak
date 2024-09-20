use std::collections::HashMap;

use eyre::ContextCompat;
use reqwest::{header::HeaderMap, Url};
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
    server: Url,
    client: ClientWithMiddleware,
    checks: HashMap<String, HealthchecksCheckInfo>,
}

impl HealthchecksService {
    /// builds the service from a config
    pub fn from_config(config: HealthchecksConfig) -> Self {
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(config.max_retries);

        let client = reqwest_middleware::ClientBuilder::new(
            reqwest::ClientBuilder::new()
                .user_agent(format!("xenbakd/{}", env!("CARGO_PKG_VERSION")))
                .build()
                .unwrap(),
        )
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

        HealthchecksService {
            config: config.clone(),
            client,
            server: Url::parse(&config.server).expect("Failed to parse healthchecks.io server url"),
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

    async fn generate_slug(&self, job_name: String) -> String {
        format!("{}", job_name)
    }
}

#[async_trait::async_trait]
impl MonitoringTrait for HealthchecksService {
    async fn success(&self, job_name: String, job_stats: XenbakJobStats) -> eyre::Result<()> {
        debug!("Sending success notification for job '{}'", job_name);

        let check = self
            .checks
            .get(&self.generate_slug(job_name).await)
            .context("Check not found")?;

        let uuid = check.ping_url.split('/').last().unwrap();

        let mut url = self.server.clone();
        url.set_path(&format!("/ping/{}", uuid));
        self.client.post(url).json(&job_stats).send().await?;

        Ok(())
    }

    async fn start(&self, job_name: String) -> eyre::Result<()> {
        debug!("Sending start notification for job '{}' ", job_name);

        let check = self
            .checks
            .get(&self.generate_slug(job_name).await)
            .context("Check not found")?;

        let uuid = check.ping_url.split('/').last().unwrap();

        let mut url = self.server.clone();
        url.set_path(&format!("/ping/{}/start", uuid));
        self.client.post(url).send().await?;

        Ok(())
    }

    async fn failure(&self, job_name: String, job_stats: XenbakJobStats) -> eyre::Result<()> {
        debug!("Sending failure notification for job '{}'", job_name);

        let check = self
            .checks
            .get(&self.generate_slug(job_name).await)
            .context("Check not found")?;

        let uuid = check.ping_url.split('/').last().unwrap();

        let mut url = self.server.clone();
        url.set_path(&format!("/ping/{}/fail", uuid));
        self.client.post(url).json(&job_stats).send().await?;

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
    async fn initialize(&mut self, jobs: Vec<JobConfig>) -> eyre::Result<()>;
}

#[async_trait::async_trait]
impl HealthchecksManagementApiTrait for HealthchecksService {
    /// List all checks for the current healthchecks.io project
    async fn list_checks(
        &self,
        tag_filter: Option<Vec<String>>,
        slug_filter: Option<String>,
    ) -> eyre::Result<HealthchecksListChecksResponse> {
        let mut url = self.server.clone();
        url.set_path("/api/v2/checks");
        let mut request = self
            .client
            .get(url)
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
    async fn initialize(&mut self, jobs: Vec<JobConfig>) -> eyre::Result<()> {
        // iterate over configured jobs, update or create checks
        for job in jobs {
            let tags = vec![""].join(" ");
            let name = self.generate_slug(job.name.clone()).await;
            let slug = name.clone();
            let grace = self.config.grace;
            let schedule = job
                .schedule
                .split_whitespace()
                .skip(1)
                .collect::<Vec<&str>>()
                .join(" ");

            let mut url = self.server.clone();
            url.set_path(&format!("/api/v2/checks"));

            dbg!(&url);

            let request = HealthchecksCreateCheckRequest {
                name: name.clone(),
                tags,
                schedule,
                grace,
                timeout: 86400,
                slug,
                unique: vec!["name".into()],
            };

            dbg!(&request);

            let response: HealthchecksCheckInfo = self
                .client
                .post(url)
                .headers(self.generate_auth_header().await?)
                .json(&request)
                .send()
                .await?
                .json()
                .await?;

            dbg!(&response);

            self.checks.insert(name.clone(), response);
        }

        Ok(())
    }
}
