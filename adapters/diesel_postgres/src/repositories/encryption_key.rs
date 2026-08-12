use crate::{
    models::{config::DbPoolProvider, tenant::Tenant as TenantModel},
    schema::tenant::{self as tenant_model, dsl as tenant_dsl},
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes,
    entities::EncryptionKeyFetching,
    utils::{
        generate_dek, unwrap_dek, wrap_dek, SYSTEM_TENANT_ID,
        SYSTEM_TENANT_NAME,
    },
};
use mycelium_base::utils::errors::{fetching_err, updating_err, MappedErrors};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = EncryptionKeyFetching)]
pub struct EncryptionKeyFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl EncryptionKeyFetching for EncryptionKeyFetchingSqlDbRepository {
    #[tracing::instrument(name = "get_or_provision_dek", skip(self, kek))]
    async fn get_or_provision_dek(
        &self,
        tenant_id: Option<Uuid>,
        kek: &[u8; 32],
    ) -> Result<[u8; 32], MappedErrors> {
        let tid = tenant_id.unwrap_or(SYSTEM_TENANT_ID);
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {e}"))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let record = tenant_dsl::tenant
            .filter(tenant_model::id.eq(tid))
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch tenant DEK row: {e}"))
            })?;

        let Some(record) = record else {
            return provision_missing_tenant_dek(conn, tid, kek);
        };

        if let Some(wrapped) = record.encrypted_dek {
            let aad = tid.as_bytes();
            return unwrap_dek(&wrapped, kek, aad);
        }

        provision_and_persist_dek(conn, tid, kek)
    }
}

/// Provision a DEK for a tenant row that was not found.
///
/// The system tenant (`Uuid::nil`) is an infrastructure sentinel that stores
/// the DEK used to encrypt tenant-less secrets (e.g. webhook secrets). It is
/// not seeded by any migration, so on a fresh database it is created here on
/// first use. A missing *real* tenant remains a genuine error.
fn provision_missing_tenant_dek(
    conn: &mut diesel::PgConnection,
    tid: Uuid,
    kek: &[u8; 32],
) -> Result<[u8; 32], MappedErrors> {
    if tid != SYSTEM_TENANT_ID {
        return fetching_err(format!("Tenant not found: {tid}"))
            .with_exp_true()
            .as_error();
    }

    diesel::insert_into(tenant_dsl::tenant)
        .values((
            tenant_model::id.eq(tid),
            tenant_model::name.eq(SYSTEM_TENANT_NAME),
            tenant_model::created.eq(chrono::Utc::now()),
            tenant_model::kek_version.eq(1),
        ))
        .on_conflict(tenant_model::id)
        .do_nothing()
        .execute(conn)
        .map_err(|e| {
            updating_err(format!("Failed to seed system tenant row: {e}"))
        })?;

    provision_and_persist_dek(conn, tid, kek)
}

fn provision_and_persist_dek(
    conn: &mut diesel::PgConnection,
    tid: Uuid,
    kek: &[u8; 32],
) -> Result<[u8; 32], MappedErrors> {
    let dek = generate_dek()?;
    let aad = tid.as_bytes();
    let wrapped = wrap_dek(&dek, kek, aad)?;

    // Only the first concurrent caller wins provisioning: the NULL guard makes
    // a racing second caller a no-op instead of rekeying (and thus orphaning)
    // an already-provisioned DEK.
    let updated = diesel::update(
        tenant_dsl::tenant
            .filter(tenant_model::id.eq(tid))
            .filter(tenant_model::encrypted_dek.is_null()),
    )
    .set(tenant_model::encrypted_dek.eq(&wrapped))
    .execute(conn)
    .map_err(|e| {
        updating_err(format!("Failed to persist DEK for tenant {tid}: {e}"))
    })?;

    if updated == 1 {
        return Ok(dek);
    }

    // A concurrent caller already provisioned (or the row vanished): read and
    // unwrap the authoritative persisted key rather than returning ours.
    let wrapped_existing = tenant_dsl::tenant
        .filter(tenant_model::id.eq(tid))
        .select(tenant_model::encrypted_dek)
        .first::<Option<String>>(conn)
        .map_err(|e| {
            fetching_err(format!("Failed to re-fetch tenant DEK row: {e}"))
        })?
        .ok_or_else(|| {
            fetching_err(format!("Tenant not found: {tid}")).with_exp_true()
        })?;

    unwrap_dek(&wrapped_existing, kek, aad)
}
