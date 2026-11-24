//! Hosted login sessions live in the `token` table: `meta` holds the
//! `HostedLoginSession` JSON, found by `meta->>'sessionId'`.

use crate::{models::token::Token as TokenModel, schema::token as token_model};

use chrono::{DateTime, Local};
use diesel::{
    sql_types::Text, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl,
};
use myc_core::domain::dtos::{email::Email, hosted_login::HostedLoginSession};
use serde_json::{from_value, to_value, Value as JsonValue};
use tracing::error;

pub(super) type Rollback = diesel::result::Error;

pub(super) fn parse_session(
    record: &TokenModel,
) -> Result<HostedLoginSession, Rollback> {
    from_value(record.meta.clone()).map_err(|e| {
        error!("Error deserializing hosted login session: {}", e);
        Rollback::RollbackTransaction
    })
}

pub(super) fn serialize_session(
    session: &HostedLoginSession,
) -> Result<JsonValue, Rollback> {
    to_value(session).map_err(|e| {
        error!("Error serializing hosted login session: {}", e);
        Rollback::RollbackTransaction
    })
}

/// Load (and lock, inside a transaction) the active rows whose `meta` value
/// at `json_key_path` (a `#>>` path such as `{sessionId}`) equals `value`.
pub(super) fn load_active_records(
    conn: &mut PgConnection,
    json_key_path: &str,
    value: &str,
) -> Result<Vec<TokenModel>, Rollback> {
    let sql = format!(
        "SELECT id, expiration, meta FROM token \
         WHERE meta #>> '{json_key_path}' = $1 AND expiration > now() \
         FOR UPDATE"
    );

    diesel::sql_query(sql)
        .bind::<Text, _>(value)
        .load::<TokenModel>(conn)
}

/// Serialize writers per email for the rest of the transaction.
pub(super) fn lock_email(
    conn: &mut PgConnection,
    email: &Email,
) -> Result<(), Rollback> {
    diesel::sql_query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind::<Text, _>(email.email())
        .execute(conn)?;

    Ok(())
}

/// Active sessions holding an unused emailed code for `email`.
pub(super) fn count_pending_codes_for_email(
    conn: &mut PgConnection,
    email: &Email,
) -> Result<usize, Rollback> {
    let sql = "SELECT id, expiration, meta FROM token \
         WHERE meta->'emailCode'->'email'->>'username' = $1 \
         AND meta->'emailCode'->'email'->>'domain' = $2 \
         AND expiration > now()";

    let records = diesel::sql_query(sql)
        .bind::<Text, _>(&email.username)
        .bind::<Text, _>(&email.domain)
        .load::<TokenModel>(conn)?;

    Ok(records.len())
}

pub(super) fn find_active_session(
    conn: &mut PgConnection,
    session_id: &str,
) -> Result<Option<(i32, HostedLoginSession)>, Rollback> {
    let records = load_active_records(conn, "{sessionId}", session_id)?;

    let Some(record) = records.first() else {
        return Ok(None);
    };

    Ok(Some((record.id, parse_session(record)?)))
}

pub(super) fn save_session(
    conn: &mut PgConnection,
    id: i32,
    session: &HostedLoginSession,
) -> Result<(), Rollback> {
    diesel::update(token_model::table.find(id))
        .set(token_model::meta.eq(serialize_session(session)?))
        .execute(conn)?;

    Ok(())
}

pub(super) fn save_session_with_expiration(
    conn: &mut PgConnection,
    id: i32,
    session: &HostedLoginSession,
    expires: DateTime<Local>,
) -> Result<(), Rollback> {
    diesel::update(token_model::table.find(id))
        .set((
            token_model::meta.eq(serialize_session(session)?),
            token_model::expiration.eq(expires.naive_utc()),
        ))
        .execute(conn)?;

    Ok(())
}

pub(super) fn delete_session(
    conn: &mut PgConnection,
    id: i32,
) -> Result<(), Rollback> {
    diesel::delete(token_model::table.find(id)).execute(conn)?;

    Ok(())
}
