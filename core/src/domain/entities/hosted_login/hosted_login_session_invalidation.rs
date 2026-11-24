use crate::domain::dtos::hosted_login::{
    HostedLoginCodeCheck, HostedLoginSession,
};

use async_trait::async_trait;
use chrono::{DateTime, Local};
#[cfg(test)]
use mockall::automock;
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use shaku::Interface;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait HostedLoginSessionInvalidation: Interface + Send + Sync {
    /// Check a typed code against an active session, under a write lock.
    ///
    /// Applies `HostedLoginSession::check_code` and persists the outcome:
    /// verified → store `authorization_code` and move the expiration to
    /// `authorization_code_expires`; rejected → store the attempt count;
    /// exhausted → store the attempt count and keep the row until it
    /// expires, so it keeps counting against the per-email cap.
    async fn check_code_and_issue_authorization_code(
        &self,
        session_id: &str,
        code: &str,
        authorization_code: String,
        authorization_code_expires: DateTime<Local>,
    ) -> Result<HostedLoginCodeCheck, MappedErrors>;

    /// Delete the active session holding `authorization_code` and return it.
    /// Single use: a second call with the same code finds nothing.
    async fn consume_authorization_code(
        &self,
        authorization_code: &str,
    ) -> Result<FetchResponseKind<HostedLoginSession, String>, MappedErrors>;
}
