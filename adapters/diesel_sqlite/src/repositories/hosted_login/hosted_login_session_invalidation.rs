use super::hosted_login_session_record::{
    delete_session, find_active_session, load_active_records, parse_session,
    save_session, save_session_with_expiration, Rollback,
};
use crate::config::SqliteDbPoolProvider;

use async_trait::async_trait;
use chrono::{DateTime, Local};
use diesel::SqliteConnection;
use myc_core::domain::{
    dtos::{
        hosted_login::{
            HostedLoginCodeCheck, HostedLoginCodeOutcome, HostedLoginSession,
        },
        native_error_codes::NativeErrorCodes,
    },
    entities::HostedLoginSessionInvalidation,
};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = HostedLoginSessionInvalidation)]
pub struct HostedLoginSessionInvalidationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl HostedLoginSessionInvalidation
    for HostedLoginSessionInvalidationSqlDbRepository
{
    #[tracing::instrument(name = "check_hosted_login_code", skip_all)]
    async fn check_code_and_issue_authorization_code(
        &self,
        session_id: &str,
        code: &str,
        authorization_code: String,
        authorization_code_expires: DateTime<Local>,
    ) -> Result<HostedLoginCodeCheck, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        // IMMEDIATE so parallel guesses queue on the attempt counter.
        conn.immediate_transaction(|conn| {
            check_code(
                conn,
                session_id,
                code,
                authorization_code,
                authorization_code_expires,
            )
        })
        .or_else(|e| {
            fetching_err(format!(
                "Unexpected error on checking hosted login code: {}",
                e
            ))
            .as_error()
        })
    }

    #[tracing::instrument(name = "consume_hosted_login_code", skip_all)]
    async fn consume_authorization_code(
        &self,
        authorization_code: &str,
    ) -> Result<FetchResponseKind<HostedLoginSession, String>, MappedErrors>
    {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        conn.immediate_transaction(|conn| consume(conn, authorization_code))
            .or_else(|e| {
                fetching_err(format!(
                    "Unexpected error on consuming authorization code: {}",
                    e
                ))
                .as_error()
            })
    }
}

fn check_code(
    conn: &mut SqliteConnection,
    session_id: &str,
    code: &str,
    authorization_code: String,
    authorization_code_expires: DateTime<Local>,
) -> Result<HostedLoginCodeCheck, Rollback> {
    let Some((id, mut session)) = find_active_session(conn, session_id)? else {
        return Ok(HostedLoginCodeCheck::NotFound);
    };

    match session.check_code(code, authorization_code) {
        HostedLoginCodeOutcome::Verified => {
            save_session_with_expiration(
                conn,
                id,
                &session,
                authorization_code_expires,
            )?;
            Ok(HostedLoginCodeCheck::Verified(session))
        }
        HostedLoginCodeOutcome::Rejected => {
            save_session(conn, id, &session)?;
            Ok(HostedLoginCodeCheck::Rejected(session))
        }
        // Kept until it expires so it still counts against the per-email cap
        HostedLoginCodeOutcome::Exhausted => {
            save_session(conn, id, &session)?;
            Ok(HostedLoginCodeCheck::NotFound)
        }
        HostedLoginCodeOutcome::NoCodeSent => {
            Ok(HostedLoginCodeCheck::NotFound)
        }
    }
}

fn consume(
    conn: &mut SqliteConnection,
    authorization_code: &str,
) -> Result<FetchResponseKind<HostedLoginSession, String>, Rollback> {
    let records =
        load_active_records(conn, "$.authorization.code", authorization_code)?;

    let Some(record) = records.first() else {
        return Ok(FetchResponseKind::NotFound(None));
    };

    let session = parse_session(record)?;
    delete_session(conn, record.id)?;

    Ok(FetchResponseKind::Found(session))
}
