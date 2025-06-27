use crate::models::{ClientProvider, QueueConfig, SmtpConfig};

use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};
use myc_adapters_shared_lib::models::{
    RedisClientWrapper, RedisConfig, SharedClientProvider,
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use redis::Client;
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = ClientProvider)]
#[derive(Clone)]
pub struct NotifierClientImpl {
    #[shaku(inject)]
    redis_client: Arc<dyn SharedClientProvider>,
    smtp_client: Arc<SmtpTransport>,
    queue_config: Arc<QueueConfig>,
}

impl ClientProvider for NotifierClientImpl {
    fn get_queue_config(&self) -> Arc<QueueConfig> {
        self.queue_config.clone()
    }

    fn get_smtp_client(&self) -> Arc<SmtpTransport> {
        self.smtp_client.clone()
    }

    fn get_redis_config(&self) -> Arc<RedisConfig> {
        self.redis_client.get_redis_config().clone()
    }

    fn get_redis_client(&self) -> Arc<Client> {
        self.redis_client.get_redis_client().clone()
    }
}

impl NotifierClientImpl {
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

        let queue_client = RedisClientWrapper::new(
            Client::open(queue_url).map_err(|err| {
                execution_err(format!("Failed to connect to queue: {err}"))
            })?,
            redis_config,
        );

        let host = smtp_config.host.async_get_or_error().await?;
        let username = smtp_config.username.async_get_or_error().await?;
        let password = smtp_config.password.async_get_or_error().await?;
        let credentials = Credentials::new(username, password);
        let port = smtp_config.port.async_get_or_error().await?;

        let smtp_client = SmtpTransport::relay(&host)
            .map_err(|err| {
                execution_err(format!("Failed to connect to SMTP: {err}"))
            })?
            .credentials(credentials)
            .port(port)
            .build();

        Ok(Arc::new(Self {
            redis_client: Arc::new(queue_client),
            smtp_client: Arc::new(smtp_client),
            queue_config: Arc::new(queue_config),
        }))
    }
}
