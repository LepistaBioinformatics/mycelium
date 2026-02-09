use crate::models::config::DbPoolProvider;

use myc_core::domain::dtos::{
    callback::{Callback, CallbackExecutor, ExecutionMode},
    service::Service,
};
use shaku::Component;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Component)]
#[shaku(interface = DbPoolProvider)]
pub struct MemDbPoolProvider {
    #[shaku(default)]
    pub services_db: Arc<Mutex<Vec<Service>>>,

    #[shaku(default)]
    pub callbacks_db: Arc<Mutex<Vec<Callback>>>,

    #[shaku(default)]
    pub engines: Arc<Mutex<Vec<Arc<dyn CallbackExecutor>>>>,

    #[shaku(default)]
    pub mode: Arc<Mutex<ExecutionMode>>,
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

    fn get_callbacks_db(&self) -> Vec<Callback> {
        self.callbacks_db.lock().unwrap().clone()
    }

    fn get_engines(&self) -> Vec<Arc<dyn CallbackExecutor>> {
        self.engines.lock().unwrap().clone()
    }

    fn get_engines_by_names(
        &self,
        callback_names: &[String],
    ) -> Vec<Arc<dyn CallbackExecutor>> {
        let engines = self.engines.lock().unwrap();
        let callbacks = self.callbacks_db.lock().unwrap();

        // Create a mapping of callback name to engine index
        // Assumes engines are created in the same order as callbacks
        let mut engine_map: HashMap<String, usize> = HashMap::new();
        for (idx, callback) in callbacks.iter().enumerate() {
            if idx < engines.len() {
                engine_map.insert(callback.name.clone(), idx);
            }
        }

        // Filter engines by callback names
        callback_names
            .iter()
            .filter_map(|name| {
                engine_map
                    .get(name)
                    .and_then(|&idx| engines.get(idx).cloned())
            })
            .collect()
    }

    fn get_execution_mode(&self) -> ExecutionMode {
        self.mode.lock().unwrap().clone()
    }
}

impl Default for MemDbPoolProvider {
    fn default() -> Self {
        Self {
            services_db: Arc::new(Mutex::new(vec![])),
            callbacks_db: Arc::new(Mutex::new(vec![])),
            engines: Arc::new(Mutex::new(vec![])),
            mode: Arc::new(Mutex::new(ExecutionMode::default())),
        }
    }
}

/* impl MemDbPoolProvider {
    pub async fn new(
        routes: Vec<Service>,
        callbacks: Option<Vec<Callback>>,
    ) -> Self {
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
            callbacks_db: Arc::new(Mutex::new(callbacks.unwrap_or_default())),
        }
    }
} */
