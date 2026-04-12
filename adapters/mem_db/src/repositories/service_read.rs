use crate::models::config::DbPoolProvider;

use async_trait::async_trait;
use myc_core::domain::{dtos::service::Service, entities::ServiceRead};
use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
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
    async fn list_services_paginated(
        &self,
        id: Option<Uuid>,
        name: Option<String>,
        discoverable: Option<bool>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<Service>, MappedErrors> {
        let db = self.db_config.get_services_db();

        if db.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let page_size = page_size.unwrap_or(10) as i64;
        let skip = skip.unwrap_or(0) as i64;

        let mut binding_db = db.clone();
        binding_db.sort_by_key(|service| service.name.clone());

        let response = binding_db
            .into_iter()
            .filter(|service| {
                //
                // Check the match between the registered service id and the
                // requested service id.
                //
                if let Some(id) = &id {
                    service.id == *id
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
            .skip(skip as usize)
            .take(page_size as usize)
            .collect::<Vec<Service>>()
            //
            // Filter unique services
            //
            .into_iter()
            .collect::<Vec<Service>>();

        if response.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::FoundPaginated {
            count: response.len() as i64,
            skip: Some(skip),
            size: Some(page_size),
            records: response,
        })
    }

    async fn list_services(
        &self,
        id: Option<Uuid>,
        name: Option<String>,
        discoverable: Option<bool>,
    ) -> Result<FetchManyResponseKind<Service>, MappedErrors> {
        let db = self.db_config.get_services_db();

        if db.len() == 0 {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let response = db
            .into_iter()
            .filter(|service| {
                //
                // Check the match between the registered service id and the
                // requested service id.
                //
                if let Some(id) = &id {
                    service.id == *id
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

#[cfg(test)]
mod tests {
    use super::*;
    use myc_core::domain::dtos::{
        callback::{Callback, CallbackExecutor, ExecutionMode},
        service::Service,
    };
    use std::sync::Arc;

    // ? -----------------------------------------------------------------------
    // ? Minimal DbPoolProvider stub for testing
    // ? -----------------------------------------------------------------------

    struct StubDbPool {
        services: Vec<Service>,
    }

    impl DbPoolProvider for StubDbPool {
        fn get_services_db(&self) -> Vec<Service> {
            self.services.clone()
        }

        fn get_services_db_mut(&self) -> Vec<Service> {
            self.services.clone()
        }

        fn set_services_db(&self, _db: Vec<Service>) {}

        fn get_callbacks_db(&self) -> Vec<Callback> {
            vec![]
        }

        fn get_engines(&self) -> Vec<Arc<dyn CallbackExecutor>> {
            vec![]
        }

        fn get_engines_by_names(
            &self,
            _callback_names: &[String],
        ) -> Vec<Arc<dyn CallbackExecutor>> {
            vec![]
        }

        fn get_execution_mode(&self) -> ExecutionMode {
            ExecutionMode::default()
        }
    }

    fn repo_with_empty_db() -> ServiceReadMemDbRepo {
        ServiceReadMemDbRepo {
            db_config: Arc::new(StubDbPool { services: vec![] }),
        }
    }

    // ? -----------------------------------------------------------------------
    // ? list_services
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_list_services_returns_not_found_when_db_is_empty(
    ) -> Result<(), MappedErrors> {
        let repo = repo_with_empty_db();

        let result = repo.list_services(None, None, None).await?;

        assert!(
            matches!(result, FetchManyResponseKind::NotFound),
            "expected NotFound when service DB is empty, got {:?}",
            result
        );

        Ok(())
    }

    // ? -----------------------------------------------------------------------
    // ? list_services_paginated
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_list_services_paginated_returns_not_found_when_db_is_empty(
    ) -> Result<(), MappedErrors> {
        let repo = repo_with_empty_db();

        let result = repo
            .list_services_paginated(None, None, None, None, None)
            .await?;

        assert!(
            matches!(result, FetchManyResponseKind::NotFound),
            "expected NotFound when service DB is empty, got {:?}",
            result
        );

        Ok(())
    }
}
