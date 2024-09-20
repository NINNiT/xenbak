use crate::{config::MailConfig, jobs::XenbakJobStats};

use lettre::{AsyncSmtpTransport, AsyncTransport};

use super::MonitoringTrait;

#[derive(Debug, Clone)]
pub struct MailService {
    from: String,
    to: String,
    mailer: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
}

impl MailService {
    pub async fn from_config(config: MailConfig) -> eyre::Result<Self> {
        // create mailer
        let mut mailer = AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(&config.smtp_server)?
            .port(config.smtp_port)
            .tls(lettre::transport::smtp::client::Tls::None);
        match (config.smtp_user.as_str(), config.smtp_password.as_str()) {
            ("", "") => (),
            (user, pass) => {
                mailer =
                    mailer.credentials(lettre::transport::smtp::authentication::Credentials::new(
                        user.to_string(),
                        pass.to_string(),
                    ))
            }
        };
        let mailer = mailer.build();

        // create recipient list from vec
        let to = config.smtp_to.join(", ");

        // build this struct
        let mail_service = MailService {
            mailer,
            from: config.smtp_from,
            to,
        };

        // test connection
        mail_service.test_conn().await?;

        Ok(mail_service)
    }

    pub async fn test_conn(&self) -> eyre::Result<()> {
        match self.mailer.test_connection().await {
            Ok(_) => Ok(()),
            Err(e) => Err(eyre::eyre!("Failed to connect to SMTP server: {}", e)),
        }
    }
}

#[async_trait::async_trait]
impl MonitoringTrait for MailService {
    async fn start(&self, _hostname: String, _job_name: String) -> eyre::Result<()> {
        // mail service, do nothing!
        Ok(())
    }
    // Method to send an email
    async fn success(
        &self,
        job_name: String,
        hostname: String,
        job_stats: XenbakJobStats,
    ) -> eyre::Result<()> {
        // pretty print the job_stats object
        let job_stats = serde_json::to_string_pretty(&job_stats)?;

        let body = format!(
            "Backup Job '{}' on host '{}' succeeded.\n\nStats: {}",
            job_name, hostname, job_stats
        );

        let email = lettre::Message::builder()
            .from(self.from.parse()?)
            .to(self.to.parse()?)
            .subject(format!("Success: Backup Job '{}' on host '{}'", job_name, hostname).as_str())
            .body(body)?;

        match self.mailer.send(email).await {
            Ok(_) => Ok(()),
            Err(e) => Err(eyre::eyre!("Failed to send email: {}", e)),
        }
    }

    async fn failure(
        &self,
        job_name: String,
        hostname: String,
        job_stats: XenbakJobStats,
    ) -> eyre::Result<()> {
        let job_stats = serde_json::to_string_pretty(&job_stats)?;
        let body = format!(
            "Backup Job '{}' on host '{}' has failed\n\nStats: {}",
            job_name, hostname, job_stats
        );

        let email = lettre::Message::builder()
            .from(self.from.parse()?)
            .to(self.to.parse()?)
            .subject(format!("Failure: Backup Job '{}' on host '{}'", job_name, hostname).as_str())
            .body(body)?;

        match self.mailer.send(email).await {
            Ok(_) => Ok(()),
            Err(e) => Err(eyre::eyre!("Failed to send email: {}", e)),
        }
    }
}
