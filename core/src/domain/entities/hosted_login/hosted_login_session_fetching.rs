use crate::domain::dtos::hosted_login::HostedLoginSession;

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Interface;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait HostedLoginSessionFetching: Interface + Send + Sync {
    /// Fetch a session that has not expired.
    async fn get_by_session_id(
        &self,
        session_id: &str,
    ) -> Result<FetchResponseKind<HostedLoginSession, String>, MappedErrors>;
}
