use crate::models::config::DbPoolProvider;

use async_trait::async_trait;
use myc_core::domain::{dtos::service::Service, entities::ServiceRead};
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = ServiceRead)]
pub struct ServiceReadMemDbRepo {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl ServiceRead for ServiceReadMemDbRepo {
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<Service, String>, MappedErrors> {
        let db = self.db_config.get_services_db();

        if db.len() == 0 {
            return fetching_err("Routes already not initialized.").as_error();
        }

        let response = db
            .into_iter()
            .filter(|service| service.id == Some(id))
            .collect::<Vec<Service>>();

        if response.len() == 0 {
            return Ok(FetchResponseKind::NotFound(None));
        }

        if response.len() > 1 {
            return fetching_err(
                "Multiple services found for the specified id.",
            )
            .with_exp_true()
            .as_error();
        }

        Ok(FetchResponseKind::Found(
            response.first().unwrap().to_owned(),
        ))
    }

    async fn list_services(
        &self,
        id: Option<Uuid>,
        name: Option<String>,
        discoverable: Option<bool>,
    ) -> Result<FetchManyResponseKind<Service>, MappedErrors> {
        let db = self.db_config.get_services_db();

        if db.len() == 0 {
            return fetching_err("Routes already not initialized.").as_error();
        }

        let response = db
            .into_iter()
            .filter(|service| {
                //
                // Check the match between the registered service id and the
                // requested service id.
                //
                if let Some(id) = &id {
                    service.id == Some(*id)
                } else {
                    true
                }
            })
            .filter(|service| {
                //
                // Check the match between the registered service name and the
                // requested service name.
                //
                if let Some(name) = &name {
                    service.name == *name
                } else {
                    true
                }
            })
            .filter(|service| {
                //
                // Check the match between the registered service discoverable
                // and the requested discoverable.
                //
                if let Some(discoverable) = &discoverable {
                    service.discoverable.unwrap_or(false) == *discoverable
                } else {
                    true
                }
            })
            .collect::<Vec<Service>>()
            //
            // Filter unique services
            //
            .into_iter()
            .collect::<Vec<Service>>();

        if response.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::Found(response))
    }
}
