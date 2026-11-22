use super::map_link_row::map_link_row_to_dto;
use crate::{
    models::federation_link::IdentityLinkRow,
    schema::federation_identity_link as link_model,
};

use chrono::Utc;
use diesel::{prelude::*, PgConnection};
use myc_core::domain::dtos::federation::{
    IdentityLink, LinkStatus, NewIdentityLink, SubjectRef,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};

/// Insert the link with a fresh `local_subject_ref`; a live link of the same
/// user and peer is a conflict (partial unique index), never a second row
pub(super) fn record_link(
    conn: &mut PgConnection,
    link: NewIdentityLink,
) -> Result<CreateResponseKind<IdentityLink>, MappedErrors> {
    let Some(row) = insert_link(conn, &link)? else {
        return already_linked(conn, link);
    };

    Ok(CreateResponseKind::Created(map_link_row_to_dto(row)?))
}

fn insert_link(
    conn: &mut PgConnection,
    link: &NewIdentityLink,
) -> Result<Option<IdentityLinkRow>, MappedErrors> {
    let established_at = (link.status == LinkStatus::Active).then(Utc::now);

    diesel::insert_into(link_model::table)
        .values((
            link_model::local_user_id.eq(link.local_user_id),
            link_model::local_email.eq(&link.local_email),
            link_model::peer_id.eq(link.peer_id),
            link_model::peer_subject_ref
                .eq(link.peer_subject_ref.as_ref().map(SubjectRef::as_str)),
            link_model::local_subject_ref.eq(SubjectRef::mint().to_string()),
            link_model::status.eq(link.status.as_str()),
            link_model::established_at.eq(established_at),
        ))
        .on_conflict_do_nothing()
        .returning(IdentityLinkRow::as_returning())
        .get_result::<IdentityLinkRow>(conn)
        .optional()
        .map_err(|e| creation_err(format!("Failed to record link: {e}")))
}

fn already_linked(
    conn: &mut PgConnection,
    link: NewIdentityLink,
) -> Result<CreateResponseKind<IdentityLink>, MappedErrors> {
    let existing = link_model::table
        .filter(link_model::local_user_id.eq(link.local_user_id))
        .filter(link_model::peer_id.eq(link.peer_id))
        .filter(link_model::status.ne(LinkStatus::Revoked.as_str()))
        .select(IdentityLinkRow::as_select())
        .first::<IdentityLinkRow>(conn)
        .map_err(|e| creation_err(format!("Failed to fetch link: {e}")))?;

    Ok(CreateResponseKind::NotCreated(
        map_link_row_to_dto(existing)?,
        "the user is already linked to this peer".to_string(),
    ))
}
