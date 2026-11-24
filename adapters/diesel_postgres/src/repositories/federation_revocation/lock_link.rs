use crate::schema::federation_identity_link as link_model;

use diesel::prelude::*;
use uuid::Uuid;

/// Lock the link's row for the transaction, so a tombstone and an assertion
/// of the same link are written one after the other (FED-23)
pub(crate) fn lock_link(
    conn: &mut PgConnection,
    link_id: Uuid,
) -> Result<(), diesel::result::Error> {
    link_model::table
        .find(link_id)
        .select(link_model::id)
        .for_update()
        .first::<Uuid>(conn)
        .optional()
        .map(|_| ())
}
