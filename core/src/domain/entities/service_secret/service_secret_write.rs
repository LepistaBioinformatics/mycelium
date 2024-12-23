use crate::domain::dtos::service_secret::ServiceSecret;

use async_trait::async_trait;
use mycelium_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};

#[async_trait]
pub trait RouteSecretWrite: Interface + Send + Sync {
    async fn create(
        &self,
        service_secret: ServiceSecret,
    ) -> Result<GetOrCreateResponseKind<ServiceSecret>, MappedErrors>;
}

impl Display for dyn RouteSecretWrite {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}

impl Debug for dyn RouteSecretWrite {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}
