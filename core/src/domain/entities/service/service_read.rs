use crate::domain::dtos::service::Service;

use async_trait::async_trait;
use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};
use uuid::Uuid;

#[async_trait]
pub trait ServiceRead: Interface + Send + Sync {
    async fn list_services(
        &self,
        id: Option<Uuid>,
        name: Option<String>,
        discoverable: Option<bool>,
    ) -> Result<FetchManyResponseKind<Service>, MappedErrors>;
}

impl Display for dyn ServiceRead {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}

impl Debug for dyn ServiceRead {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}
