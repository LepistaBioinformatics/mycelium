use crate::models::federation_link::{LinkConsentRow, LinkRequestRow};

use chrono::Local;
use myc_core::domain::dtos::federation::{LinkConsent, LinkRequest};
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use uuid::Uuid;

pub(super) fn map_request_row(row: LinkRequestRow) -> LinkRequest {
    LinkRequest {
        id: row.id,
        local_user_id: row.local_user_id,
        peer_id: row.peer_id,
        expires_at: row.expires_at.with_timezone(&Local),
        redirected_at: row.redirected_at.map(|at| at.with_timezone(&Local)),
    }
}

pub(super) fn map_consent_row(row: LinkConsentRow) -> LinkConsent {
    LinkConsent {
        id: row.id,
        peer_id: row.peer_id,
        peer_request_id: row.peer_request_id,
        peer_email: row.peer_email,
        return_url: row.return_url,
        local_user_id: row.local_user_id,
        expires_at: row.expires_at.with_timezone(&Local),
    }
}

/// `Found` with the mapped row, or `NotFound` naming the id
pub(super) fn found<R, T: serde::Serialize>(
    row: Option<R>,
    id: Uuid,
    map: fn(R) -> T,
) -> Result<FetchResponseKind<T, Uuid>, MappedErrors> {
    let Some(row) = row else {
        return Ok(FetchResponseKind::NotFound(Some(id)));
    };

    Ok(FetchResponseKind::Found(map(row)))
}
