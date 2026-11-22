use super::map_link_row::map_link_row_to_dto;
use crate::{
    models::federation_link::IdentityLinkRow,
    schema::federation_identity_link as link_model,
};

use chrono::Utc;
use diesel::{prelude::*, PgConnection};
use myc_core::domain::dtos::{
    federation::{IdentityLink, LinkStatus, SubjectRef},
    written_by::WrittenBy,
};
use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{updating_err, MappedErrors},
};
use uuid::Uuid;

/// Pending → active, in one conditional `UPDATE`
pub(super) fn activate_link(
    conn: &mut PgConnection,
    link_id: Uuid,
    peer_subject_ref: SubjectRef,
) -> Result<UpdatingResponseKind<IdentityLink>, MappedErrors> {
    let activated = diesel::update(
        link_model::table
            .find(link_id)
            .filter(link_model::status.eq(LinkStatus::Pending.as_str())),
    )
    .set((
        link_model::status.eq(LinkStatus::Active.as_str()),
        link_model::peer_subject_ref.eq(Some(peer_subject_ref.to_string())),
        link_model::established_at.eq(Some(Utc::now())),
    ))
    .returning(IdentityLinkRow::as_returning())
    .get_result::<IdentityLinkRow>(conn)
    .optional()
    .map_err(|e| updating_err(format!("Failed to activate link: {e}")))?;

    settled(conn, link_id, activated, "the link is not pending")
}

/// Any live status → revoked, in one conditional `UPDATE`
pub(super) fn revoke_link(
    conn: &mut PgConnection,
    link_id: Uuid,
    revoked_by: Option<WrittenBy>,
) -> Result<UpdatingResponseKind<IdentityLink>, MappedErrors> {
    let revoked_by = revoked_by
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| updating_err(format!("Invalid revoking author: {e}")))?;

    let revoked = diesel::update(
        link_model::table
            .find(link_id)
            .filter(link_model::status.ne(LinkStatus::Revoked.as_str())),
    )
    .set((
        link_model::status.eq(LinkStatus::Revoked.as_str()),
        link_model::revoked_at.eq(Some(Utc::now())),
        link_model::revoked_by.eq(revoked_by),
    ))
    .returning(IdentityLinkRow::as_returning())
    .get_result::<IdentityLinkRow>(conn)
    .optional()
    .map_err(|e| updating_err(format!("Failed to revoke link: {e}")))?;

    settled(conn, link_id, revoked, "the link is already revoked")
}

fn settled(
    conn: &mut PgConnection,
    link_id: Uuid,
    changed: Option<IdentityLinkRow>,
    reason: &str,
) -> Result<UpdatingResponseKind<IdentityLink>, MappedErrors> {
    if let Some(row) = changed {
        return Ok(UpdatingResponseKind::Updated(map_link_row_to_dto(row)?));
    }

    let stored = link_model::table
        .find(link_id)
        .select(IdentityLinkRow::as_select())
        .first::<IdentityLinkRow>(conn)
        .map_err(|e| updating_err(format!("link {link_id} not found: {e}")))?;

    Ok(UpdatingResponseKind::NotUpdated(
        map_link_row_to_dto(stored)?,
        reason.to_string(),
    ))
}
