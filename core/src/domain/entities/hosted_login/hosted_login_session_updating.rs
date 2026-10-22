use crate::domain::dtos::hosted_login::{
    HostedLoginCodeAttachment, HostedLoginEmailCode,
};

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait HostedLoginSessionUpdating: Interface + Send + Sync {
    /// Attach the emailed code to an active session.
    ///
    /// Refuses when the session already got a code or was verified
    /// (`AlreadySent`), or when `max_pending_per_email` active sessions
    /// already hold an unused code for the same email (`EmailCapReached`).
    /// The count and the write must run under a lock keyed on the email, so
    /// parallel requests cannot all pass the cap.
    async fn attach_email_code(
        &self,
        session_id: &str,
        email_code: HostedLoginEmailCode,
        max_pending_per_email: u8,
    ) -> Result<HostedLoginCodeAttachment, MappedErrors>;
}
