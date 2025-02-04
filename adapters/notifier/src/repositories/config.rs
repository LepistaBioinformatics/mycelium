use crate::models::{ClientProvider, QueueConfig, SmtpConfig};

use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use redis::Client;
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = ClientProvider)]
#[derive(Clone)]
pub struct RedisClientProvider {
    queue_client: Arc<Client>,
    smtp_client: Arc<SmtpTransport>,
    config: Arc<QueueConfig>,
}

impl ClientProvider for RedisClientProvider {
    fn get_queue_client(&self) -> Arc<Client> {
        self.queue_client.clone()
    }

    fn get_smtp_client(&self) -> Arc<SmtpTransport> {
        self.smtp_client.clone()
    }

    fn get_config(&self) -> Arc<QueueConfig> {
        self.config.clone()
    }
}

impl RedisClientProvider {
    pub async fn new(
        queue_config: QueueConfig,
        smtp_config: SmtpConfig,
    ) -> Result<Arc<Self>, MappedErrors> {
        let queue_url = format!(
            "{}://:{}@{}",
            queue_config.protocol.async_get_or_error().await?,
            queue_config.password.async_get_or_error().await?,
            queue_config.hostname.async_get_or_error().await?
        );

        let queue_client = Client::open(queue_url).map_err(|err| {
            execution_err(format!("Failed to connect to queue: {err}"))
        })?;

        let host = smtp_config.host.async_get_or_error().await?;
        let username = smtp_config.username.async_get_or_error().await?;
        let password = smtp_config.password.async_get_or_error().await?;
        let credentials = Credentials::new(username, password);

        let smtp_client = SmtpTransport::relay(&host)
            .map_err(|err| {
                execution_err(format!("Failed to connect to SMTP: {err}"))
            })?
            .credentials(credentials)
            .build();

        Ok(Arc::new(Self {
            queue_client: Arc::new(queue_client),
            smtp_client: Arc::new(smtp_client),
            config: Arc::new(queue_config),
        }))
    }
}
