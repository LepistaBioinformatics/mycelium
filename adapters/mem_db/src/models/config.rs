use myc_core::domain::dtos::service::Service;
use serde::Deserialize;
use shaku::Interface;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemDbConfig {
    pub routes: Option<String>,
}

pub trait DbPoolProvider: Interface + Send + Sync {
    fn get_services_db(&self) -> Vec<Service>;
    fn get_services_db_mut(&self) -> Vec<Service>;
    fn set_services_db(&self, db: Vec<Service>);
}
