use crate::domain::dtos::guest::GuestRoleDTO;

use agrobase::{
    entities::default_response::DeletionResponseKind,
    utils::errors::MappedErrors,
};
use async_trait::async_trait;
use shaku::Interface;
use uuid::Uuid;

#[async_trait]
pub trait GuestRoleDeletion: Interface + Send + Sync {
    async fn delete(
        &self,
        user_role_id: Uuid,
    ) -> Result<DeletionResponseKind<GuestRoleDTO>, MappedErrors>;
}
