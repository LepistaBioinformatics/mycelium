use crate::models::config::DbPoolProvider;

use myc_core::{
    domain::dtos::service::Service,
    use_cases::gateway::routes::load_config_from_yaml,
};
use shaku::Component;

// ? ---------------------------------------------------------------------------
// ? Configure routes and profile
//
// Here routes and profile services are loaded.
// ? ---------------------------------------------------------------------------

#[derive(Component)]
#[shaku(interface = DbPoolProvider)]
#[derive(Debug, Clone)]
pub struct MemDbPoolProvider {
    db: Vec<Service>,
}

impl DbPoolProvider for MemDbPoolProvider {
    fn get_services_db(&self) -> Vec<Service> {
        self.db.clone()
    }
}

impl MemDbPoolProvider {
    pub async fn new(routes: Option<String>) -> Self {
        let source_file_path = match routes.clone() {
            None => {
                tracing::info!("Routes file not provided. Initializing in memory routes without downstream services.");
                return Self { db: vec![] };
            }
            Some(path) => path,
        };

        let db = load_config_from_yaml(source_file_path)
            .await
            .map_err(|err| {
                panic!("Unexpected error on load in memory database: {err}")
            })
            .unwrap();

        Self { db }
    }
}
