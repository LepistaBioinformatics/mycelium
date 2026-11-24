use crate::models::federation_assertion::AssertionRow;

use chrono::Local;
use myc_core::domain::dtos::federation::{
    StoredAssertion, VerifiedAssertion, PROFILE_ASSERTION_KIND,
};
use mycelium_base::utils::errors::{fetching_err, MappedErrors};

pub(super) fn map_assertion_row_to_dto(
    row: AssertionRow,
) -> Result<StoredAssertion, MappedErrors> {
    let payload = serde_json::from_value(row.payload).map_err(|e| {
        fetching_err(format!("Stored assertion payload is invalid: {e}"))
    })?;

    Ok(StoredAssertion {
        link_id: row.link_id,
        peer_id: row.peer_id,
        assertion: VerifiedAssertion {
            jws: row.jws,
            payload,
            issued_at: row.iat.with_timezone(&Local),
            expires_at: row.exp.with_timezone(&Local),
        },
    })
}

pub(super) fn map_assertion_dto_to_row(
    stored: StoredAssertion,
) -> Result<AssertionRow, MappedErrors> {
    let payload =
        serde_json::to_value(&stored.assertion.payload).map_err(|e| {
            fetching_err(format!("Assertion payload cannot be stored: {e}"))
        })?;

    Ok(AssertionRow {
        link_id: stored.link_id,
        peer_id: stored.peer_id,
        kind: PROFILE_ASSERTION_KIND.to_string(),
        jws: stored.assertion.jws,
        payload,
        iat: stored.assertion.issued_at.to_utc(),
        exp: stored.assertion.expires_at.to_utc(),
    })
}
