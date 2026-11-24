use crate::domain::dtos::hosted_login::HostedLoginSession;

use async_trait::async_trait;
use chrono::{DateTime, Local};
#[cfg(test)]
use mockall::automock;
use mycelium_base::{
    entities::CreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait HostedLoginSessionRegistration: Interface + Send + Sync {
    async fn create(
        &self,
        session: HostedLoginSession,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<HostedLoginSession>, MappedErrors>;
}
