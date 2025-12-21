use crate::models::config::DbPoolProvider;

use myc_core::domain::dtos::service::Service;
use shaku::Component;
use std::{
    mem::size_of_val,
    sync::{Arc, Mutex},
};

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
    pub async fn new(routes: Vec<Service>) -> Self {
        let db = match routes.clone() {
            routes => routes,
        };

        println!(
            "Local service configuration successfully loaded:\n
        Number of services: {}
        In memory size: {:.6} Mb\n",
            db.len(),
            ((size_of_val(&*db) as f64 * 0.000001) as f64),
        );

        Self {
            services_db: Arc::new(Mutex::new(db)),
        }
    }
}
