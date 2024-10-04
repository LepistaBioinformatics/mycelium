use crate::domain::dtos::tag::Tag;

use async_trait::async_trait;
use mycelium_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use std::collections::HashMap;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};
use uuid::Uuid;

#[async_trait]
pub trait TenantTagRegistration: Interface + Send + Sync {
    async fn get_or_create(
        &self,
        tenant_id: Uuid,
        tag: String,
        meta: HashMap<String, String>,
    ) -> Result<GetOrCreateResponseKind<Tag>, MappedErrors>;
}

impl Display for dyn TenantTagRegistration {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}

impl Debug for dyn TenantTagRegistration {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}
