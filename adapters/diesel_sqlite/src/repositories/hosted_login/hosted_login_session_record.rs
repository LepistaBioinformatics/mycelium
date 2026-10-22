//! Hosted login sessions live in the `token` table: `meta` holds the
//! serialized `HostedLoginSession`, found by `$.sessionId`.

use crate::{
    models::token::Token as TokenModel, schema::token,
    types::naive_timestamp_to_text,
};

use chrono::{DateTime, Local, Utc};
use diesel::{
    sql_types::Text, ExpressionMethods, QueryDsl, RunQueryDsl, SqliteConnection,
};
use myc_core::domain::dtos::{email::Email, hosted_login::HostedLoginSession};
use tracing::error;

pub(super) type Rollback = diesel::result::Error;

pub(super) fn now_as_text() -> String {
    naive_timestamp_to_text(&Utc::now().naive_utc())
}

pub(super) fn parse_session(
    record: &TokenModel,
) -> Result<HostedLoginSession, Rollback> {
    serde_json::from_str(&record.meta).map_err(|e| {
        error!("Error deserializing hosted login session: {}", e);
        Rollback::RollbackTransaction
    })
}

pub(super) fn serialize_session(
    session: &HostedLoginSession,
) -> Result<String, Rollback> {
    serde_json::to_string(session).map_err(|e| {
        error!("Error serializing hosted login session: {}", e);
        Rollback::RollbackTransaction
    })
}

/// Load the active rows matching `json_path = value`.
pub(super) fn load_active_records(
    conn: &mut SqliteConnection,
    json_path: &str,
    value: &str,
) -> Result<Vec<TokenModel>, Rollback> {
    let sql = format!(
        "SELECT id, expiration, meta FROM token \
         WHERE json_extract(meta, '{json_path}') = ? AND expiration > ?"
    );

    diesel::sql_query(sql)
        .bind::<Text, _>(value)
        .bind::<Text, _>(now_as_text())
        .load::<TokenModel>(conn)
}

/// Active sessions holding an unused emailed code for `email`.
pub(super) fn count_pending_codes_for_email(
    conn: &mut SqliteConnection,
    email: &Email,
) -> Result<usize, Rollback> {
    let sql = "SELECT id, expiration, meta FROM token \
         WHERE json_extract(meta, '$.emailCode.email.username') = ? \
         AND json_extract(meta, '$.emailCode.email.domain') = ? \
         AND expiration > ?";

    let records = diesel::sql_query(sql)
        .bind::<Text, _>(&email.username)
        .bind::<Text, _>(&email.domain)
        .bind::<Text, _>(now_as_text())
        .load::<TokenModel>(conn)?;

    Ok(records.len())
}

pub(super) fn find_active_session(
    conn: &mut SqliteConnection,
    session_id: &str,
) -> Result<Option<(i32, HostedLoginSession)>, Rollback> {
    let records = load_active_records(conn, "$.sessionId", session_id)?;

    let Some(record) = records.first() else {
        return Ok(None);
    };

    Ok(Some((record.id, parse_session(record)?)))
}

pub(super) fn save_session(
    conn: &mut SqliteConnection,
    id: i32,
    session: &HostedLoginSession,
) -> Result<(), Rollback> {
    diesel::update(token::table.find(id))
        .set(token::meta.eq(serialize_session(session)?))
        .execute(conn)?;

    Ok(())
}

pub(super) fn save_session_with_expiration(
    conn: &mut SqliteConnection,
    id: i32,
    session: &HostedLoginSession,
    expires: DateTime<Local>,
) -> Result<(), Rollback> {
    diesel::update(token::table.find(id))
        .set((
            token::meta.eq(serialize_session(session)?),
            token::expiration.eq(naive_timestamp_to_text(&expires.naive_utc())),
        ))
        .execute(conn)?;

    Ok(())
}

pub(super) fn delete_session(
    conn: &mut SqliteConnection,
    id: i32,
) -> Result<(), Rollback> {
    diesel::delete(token::table.find(id)).execute(conn)?;

    Ok(())
}
