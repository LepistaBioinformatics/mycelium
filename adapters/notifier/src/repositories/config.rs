use crate::models::{ClientProvider, QueueConfig, SmtpConfig};

use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};
use myc_adapters_shared_lib::models::RedisConfig;
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use redis::Client;
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = ClientProvider)]
#[derive(Clone)]
pub struct NotifierClientProvider {
    redis_client: Arc<Client>,
    smtp_client: Arc<SmtpTransport>,
    queue_config: Arc<QueueConfig>,
}

impl ClientProvider for NotifierClientProvider {
    fn get_redis_client(&self) -> Arc<Client> {
        self.redis_client.clone()
    }

    fn get_smtp_client(&self) -> Arc<SmtpTransport> {
        self.smtp_client.clone()
    }

    fn get_queue_config(&self) -> Arc<QueueConfig> {
        self.queue_config.clone()
    }
}

impl NotifierClientProvider {
    pub async fn new(
        queue_config: QueueConfig,
        redis_config: RedisConfig,
        smtp_config: SmtpConfig,
    ) -> Result<Arc<Self>, MappedErrors> {
        let queue_url = format!(
            "{}://:{}@{}",
            redis_config.protocol.async_get_or_error().await?,
            redis_config.password.async_get_or_error().await?,
            redis_config.hostname.async_get_or_error().await?
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
            redis_client: Arc::new(queue_client),
            smtp_client: Arc::new(smtp_client),
            queue_config: Arc::new(queue_config),
        }))
    }
}
