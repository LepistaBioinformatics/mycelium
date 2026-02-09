use myc_core::domain::dtos::{
    callback::{Callback, CallbackExecutor, ExecutionMode},
    service::Service,
};
use serde::Deserialize;
use shaku::Interface;
use std::sync::Arc;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemDbConfig {
    pub routes: Option<String>,
}

pub trait DbPoolProvider: Interface + Send + Sync {
    fn get_services_db(&self) -> Vec<Service>;
    fn get_services_db_mut(&self) -> Vec<Service>;
    fn set_services_db(&self, db: Vec<Service>);
    fn get_callbacks_db(&self) -> Vec<Callback>;
    fn get_engines(&self) -> Vec<Arc<dyn CallbackExecutor>>;
    fn get_engines_by_names(
        &self,
        callback_names: &[String],
    ) -> Vec<Arc<dyn CallbackExecutor>>;
    fn get_execution_mode(&self) -> ExecutionMode;
}
