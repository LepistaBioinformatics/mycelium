use crate::domain::dtos::tenant::{Tenant, TenantMeta, TenantMetaKey};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use serde::{Deserialize, Serialize};
use shaku::Interface;
use std::fmt::Result as FmResult;
use std::fmt::{Debug, Display, Formatter};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TenantOwnerConnection {
    pub tenant_id: Uuid,
    pub owner_id: Uuid,
    pub guest_by: String,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
}

#[async_trait]
pub trait TenantRegistration: Interface + Send + Sync {
    async fn create(
        &self,
        tenant: Tenant,
        guest_by: String,
    ) -> Result<CreateResponseKind<Tenant>, MappedErrors>;

    async fn register_tenant_meta(
        &self,
        owners_ids: Vec<Uuid>,
        tenant_id: Uuid,
        key: TenantMetaKey,
        value: String,
    ) -> Result<CreateResponseKind<TenantMeta>, MappedErrors>;
}

impl Display for dyn TenantRegistration {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}

impl Debug for dyn TenantRegistration {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmResult {
        write!(f, "{}", self)
    }
}
