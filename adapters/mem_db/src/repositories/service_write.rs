use crate::models::config::DbPoolProvider;

use async_trait::async_trait;
use myc_core::domain::{
    dtos::health_check_info::HealthStatus, entities::ServiceWrite,
};
use mycelium_base::utils::errors::{fetching_err, MappedErrors};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = ServiceWrite)]
pub struct ServiceWriteMemDbRepo {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl ServiceWrite for ServiceWriteMemDbRepo {
    #[tracing::instrument(name = "inform_health_status", skip(self))]
    async fn inform_health_status(
        &self,
        id: Uuid,
        name: String,
        health_status: HealthStatus,
    ) -> Result<(), MappedErrors> {
        let services = self.db_config.get_services_db_mut();
        let mut updated_services = services.clone();

        let service = updated_services
            .iter_mut()
            .find(|s| s.id == id && s.name == name)
            .ok_or(fetching_err("Service not found"))?;

        service.update_health_status(health_status);

        self.db_config.set_services_db(updated_services);

        Ok(())
    }
}
