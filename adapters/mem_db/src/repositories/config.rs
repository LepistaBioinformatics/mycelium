use crate::models::config::DbPoolProvider;

use myc_core::{
    domain::dtos::service::Service,
    use_cases::gateway::routes::load_config_from_yaml,
};
use shaku::Component;
use std::sync::{Arc, Mutex};

// ? ---------------------------------------------------------------------------
// ? Configure routes and profile
//
// Here routes and profile services are loaded.
// ? ---------------------------------------------------------------------------

#[derive(Component)]
#[shaku(interface = DbPoolProvider)]
pub struct MemDbPoolProvider {
    #[shaku(default)]
    pub services_db: Arc<Mutex<Vec<Service>>>,
}

impl DbPoolProvider for MemDbPoolProvider {
    fn get_services_db(&self) -> Vec<Service> {
        self.services_db.lock().unwrap().clone()
    }

    fn get_services_db_mut(&self) -> Vec<Service> {
        self.services_db.lock().unwrap().clone()
    }

    fn set_services_db(&self, services: Vec<Service>) {
        *self.services_db.lock().unwrap() = services;
    }
}

impl Default for MemDbPoolProvider {
    fn default() -> Self {
        Self {
            services_db: Arc::new(Mutex::new(vec![])),
        }
    }
}

impl MemDbPoolProvider {
    pub async fn new(routes: Option<String>) -> Self {
        let source_file_path = match routes.clone() {
            None => {
                tracing::info!("Routes file not provided. Initializing in memory routes without downstream services.");
                return Self {
                    services_db: Arc::new(Mutex::new(vec![])),
                };
            }
            Some(path) => path,
        };

        let db = load_config_from_yaml(source_file_path)
            .await
            .map_err(|err| {
                panic!("Unexpected error on load in memory database: {err}")
            })
            .unwrap();

        Self {
            services_db: Arc::new(Mutex::new(db)),
        }
    }
}
