mod check_single_host_health;
mod check_single_service_health;
mod execute_health_check_cicle;

use crate::models::api_config::ApiConfig;

use execute_health_check_cicle::{
    execute_health_check_cicle, ServiceHealthRunStatus,
};
use myc_core::domain::entities::{ServiceRead, ServiceWrite};
use myc_mem_db::repositories::MemDbAppModule;
use rand::Rng;
use shaku::HasComponent;
use std::{sync::Arc, time::Duration};

/// Check downstream services health
///
/// This function will dispatch a independent task to check the health of the
/// downstream services.
///
#[tracing::instrument(name = "services_health_dispatcher", skip_all)]
pub(crate) async fn services_health_dispatcher(
    config: ApiConfig,
    mem_app_modules: Arc<MemDbAppModule>,
) {
    tokio::spawn(tracing::Span::current().in_scope(|| async move {
        tracing::info!("Starting services health dispatcher");

        let inner_service_read_repo: &dyn ServiceRead =
            mem_app_modules.resolve_ref();
        let inner_service_write_repo: &dyn ServiceWrite =
            mem_app_modules.resolve_ref();

        let mut interval = actix_rt::time::interval(Duration::from_secs(
            config.health_check_interval.unwrap_or(60 * 5),
        ));

        tracing::trace!(
            "Services health dispatcher interval: {} seconds",
            interval.period().as_secs()
        );

        let max_retry_count = config.max_retry_count.unwrap_or(3);
        let max_instances = config.max_error_instances.unwrap_or(5);

        //
        // Skip the first tick to avoid fetching events that were created in the
        // same second as the dispatcher start.
        //
        interval.tick().await;

        //
        // Wait for a random time between 1 and the consume interval. This time
        // should avoid the webhook dispatcher to start at the same time as the
        // email dispatcher and avoid the simultaneous consumption of the same
        // event over multiple containers.
        //
        let random_time =
            rand::thread_rng().gen_range(1..=interval.period().as_secs());

        tokio::time::sleep(Duration::from_secs(random_time)).await;

        loop {
            interval.tick().await;

            let status = execute_health_check_cicle(
                max_retry_count,
                max_instances,
                Box::new(inner_service_read_repo),
                Box::new(inner_service_write_repo),
            )
            .await;

            match status {
                Ok(ServiceHealthRunStatus::Continue) => continue,
                Ok(ServiceHealthRunStatus::Stop) => break,
                Err(err) => {
                    tracing::error!(
                        "Error on check services health during services health dispatcher: {err}"
                    );
                }
            }
        }
    }));
}
