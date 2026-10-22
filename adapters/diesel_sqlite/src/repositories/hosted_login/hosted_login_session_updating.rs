use super::hosted_login_session_record::{
    count_pending_codes_for_email, find_active_session, save_session, Rollback,
};
use crate::config::SqliteDbPoolProvider;

use async_trait::async_trait;
use diesel::SqliteConnection;
use myc_core::domain::{
    dtos::{
        hosted_login::{HostedLoginCodeAttachment, HostedLoginEmailCode},
        native_error_codes::NativeErrorCodes,
    },
    entities::HostedLoginSessionUpdating,
};
use mycelium_base::utils::errors::{updating_err, MappedErrors};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = HostedLoginSessionUpdating)]
pub struct HostedLoginSessionUpdatingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl HostedLoginSessionUpdating for HostedLoginSessionUpdatingSqlDbRepository {
    #[tracing::instrument(name = "attach_hosted_login_email_code", skip_all)]
    async fn attach_email_code(
        &self,
        session_id: &str,
        email_code: HostedLoginEmailCode,
        max_pending_per_email: u8,
    ) -> Result<HostedLoginCodeAttachment, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // IMMEDIATE takes the write lock before the reads, so parallel
        // requests cannot all pass the per-email cap.
        conn.immediate_transaction(|conn| {
            attach_within_cap(
                conn,
                session_id,
                email_code,
                max_pending_per_email,
            )
        })
        .or_else(|e| {
            updating_err(format!(
                "Unexpected error on attaching hosted login code: {}",
                e
            ))
            .as_error()
        })
    }
}

fn attach_within_cap(
    conn: &mut SqliteConnection,
    session_id: &str,
    email_code: HostedLoginEmailCode,
    max_pending_per_email: u8,
) -> Result<HostedLoginCodeAttachment, Rollback> {
    let Some((id, mut session)) = find_active_session(conn, session_id)? else {
        return Ok(HostedLoginCodeAttachment::NotFound);
    };

    if session.code_already_sent() {
        return Ok(HostedLoginCodeAttachment::AlreadySent);
    }

    let pending = count_pending_codes_for_email(conn, &email_code.email)?;

    if pending >= usize::from(max_pending_per_email) {
        return Ok(HostedLoginCodeAttachment::EmailCapReached);
    }

    session.email_code = Some(email_code);
    save_session(conn, id, &session)?;

    Ok(HostedLoginCodeAttachment::Attached)
}
