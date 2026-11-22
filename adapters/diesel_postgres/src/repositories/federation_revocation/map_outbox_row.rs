use crate::models::federation_revocation::{NewOutboxRow, OutboxRow};

use chrono::{DateTime, Local, Utc};
use myc_core::domain::dtos::federation::{
    OutboxEntry, PendingRevocation, SubjectRef, Tombstone,
};
use mycelium_base::utils::errors::{fetching_err, MappedErrors};

pub(super) fn map_outbox_row_to_dto(
    row: OutboxRow,
) -> Result<OutboxEntry, MappedErrors> {
    let subject_ref = SubjectRef::try_from(row.subject_ref)
        .map_err(|e| fetching_err(format!("Invalid outbox subject: {e}")))?;
    let reason = row.reason.parse().map_err(fetching_err)?;

    Ok(OutboxEntry {
        id: row.id,
        revocation: PendingRevocation {
            peer_id: row.peer_id,
            tombstone: Tombstone {
                subject_ref,
                reason,
                issued_at: row.issued_at.with_timezone(&Local),
            },
        },
        attempts: row.attempts.max(0) as u32,
    })
}

pub(super) fn map_revocation_to_row(
    revocation: PendingRevocation,
    next_attempt_at: DateTime<Utc>,
) -> NewOutboxRow {
    NewOutboxRow {
        peer_id: revocation.peer_id,
        subject_ref: revocation.tombstone.subject_ref.as_str().to_string(),
        reason: revocation.tombstone.reason.as_str().to_string(),
        issued_at: revocation.tombstone.issued_at.with_timezone(&Utc),
        next_attempt_at,
    }
}
