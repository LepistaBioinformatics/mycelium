use crate::{
    models::federation_link::IdentityLinkRow,
    repositories::parse_optional_written_by,
};

use chrono::Local;
use myc_core::domain::dtos::federation::{
    IdentityLink, LinkStatus, SubjectRef,
};
use mycelium_base::utils::errors::{fetching_err, MappedErrors};

pub(super) fn map_link_row_to_dto(
    row: IdentityLinkRow,
) -> Result<IdentityLink, MappedErrors> {
    Ok(IdentityLink {
        id: row.id,
        local_user_id: row.local_user_id,
        local_email: row.local_email,
        peer_id: row.peer_id,
        peer_subject_ref: row.peer_subject_ref.map(subject_ref).transpose()?,
        local_subject_ref: subject_ref(row.local_subject_ref)?,
        status: status(&row.status)?,
        created: row.created.with_timezone(&Local),
        established_at: row.established_at.map(|at| at.with_timezone(&Local)),
        revoked_at: row.revoked_at.map(|at| at.with_timezone(&Local)),
        revoked_by: parse_optional_written_by(row.revoked_by),
    })
}

fn subject_ref(value: String) -> Result<SubjectRef, MappedErrors> {
    SubjectRef::try_from(value)
        .map_err(|e| fetching_err(format!("Invalid stored link: {e}")))
}

fn status(value: &str) -> Result<LinkStatus, MappedErrors> {
    value
        .parse::<LinkStatus>()
        .map_err(|e| fetching_err(format!("Invalid stored link: {e}")))
}
