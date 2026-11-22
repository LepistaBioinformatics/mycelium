use crate::models::federation_translation::TranslationRow;

use myc_core::domain::dtos::{
    federation::{
        ActiveTranslation, GrantTranslationRule, IssuerGrantMatch, LocalGrant,
        TranslationTarget,
    },
    guest_role::Permission,
};
use uuid::Uuid;

/// The rule, with its target as joined: peer instance id, account name,
/// account tenant and guest role slug
pub(crate) type ActiveTranslationRow =
    (TranslationRow, Uuid, String, Option<Uuid>, String);

fn issuer_of(row: &TranslationRow) -> IssuerGrantMatch {
    IssuerGrantMatch {
        tenant_id: row.issuer_tenant_id,
        role: row.issuer_role.clone(),
        min_perm: Permission::from_i32(row.issuer_perm),
    }
}

pub(super) fn map_translation_row_to_dto(
    row: TranslationRow,
) -> GrantTranslationRule {
    GrantTranslationRule {
        id: row.id,
        peer_id: row.peer_id,
        issuer: issuer_of(&row),
        local: LocalGrant {
            account_id: row.local_account_id,
            guest_role_id: row.local_guest_role_id,
            perm: Permission::from_i32(row.local_perm),
        },
        enabled: row.enabled,
    }
}

/// `None` for an account without a tenant: it cannot carry a guest grant
pub(super) fn map_active_row(
    (row, peer_instance_id, account_name, tenant_id, role_slug): ActiveTranslationRow,
) -> Option<ActiveTranslation> {
    Some(ActiveTranslation {
        peer_instance_id,
        issuer: issuer_of(&row),
        target: TranslationTarget {
            account_id: row.local_account_id,
            account_name,
            tenant_id: tenant_id?,
            role_id: row.local_guest_role_id,
            role_slug,
        },
        perm: Permission::from_i32(row.local_perm),
    })
}
