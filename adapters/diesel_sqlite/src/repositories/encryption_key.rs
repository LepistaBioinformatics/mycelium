use crate::{
    config::SqliteDbPoolProvider,
    models::tenant::Tenant as TenantModel,
    schema::tenant::{self, dsl as tenant_dsl},
    types::{naive_timestamp_to_text, uuid_to_text},
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
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
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
        let tid_text = uuid_to_text(&tid);

        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {e}"))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let record = tenant_dsl::tenant
            .filter(tenant::id.eq(&tid_text))
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch tenant DEK row: {e}"))
            })?;

        let Some(record) = record else {
            return provision_missing_tenant_dek(conn, &tid_text, tid, kek);
        };

        if let Some(wrapped) = record.encrypted_dek {
            let aad = tid.as_bytes();
            return unwrap_dek(&wrapped, kek, aad);
        }

        provision_and_persist_dek(conn, &tid_text, tid, kek)
    }
}

/// Provision a DEK for a tenant row that was not found.
///
/// The system tenant (`Uuid::nil`) is an infrastructure sentinel that stores
/// the DEK used to encrypt tenant-less secrets (e.g. webhook secrets). It is
/// not seeded by any migration, so on a fresh database it is created here on
/// first use. A missing *real* tenant remains a genuine error.
fn provision_missing_tenant_dek(
    conn: &mut diesel::SqliteConnection,
    tid_text: &str,
    tid: Uuid,
    kek: &[u8; 32],
) -> Result<[u8; 32], MappedErrors> {
    if tid != SYSTEM_TENANT_ID {
        return fetching_err(format!("Tenant not found: {tid}"))
            .with_exp_true()
            .as_error();
    }

    let created = naive_timestamp_to_text(&chrono::Utc::now().naive_utc());

    diesel::insert_into(tenant_dsl::tenant)
        .values((
            tenant::id.eq(tid_text),
            tenant::name.eq(SYSTEM_TENANT_NAME),
            tenant::created.eq(created),
            tenant::kek_version.eq(1),
        ))
        .on_conflict(tenant::id)
        .do_nothing()
        .execute(conn)
        .map_err(|e| {
            updating_err(format!("Failed to seed system tenant row: {e}"))
        })?;

    provision_and_persist_dek(conn, tid_text, tid, kek)
}

fn provision_and_persist_dek(
    conn: &mut diesel::SqliteConnection,
    tid_text: &str,
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
            .filter(tenant::id.eq(tid_text))
            .filter(tenant::encrypted_dek.is_null()),
    )
    .set(tenant::encrypted_dek.eq(&wrapped))
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
        .filter(tenant::id.eq(tid_text))
        .select(tenant::encrypted_dek)
        .first::<Option<String>>(conn)
        .map_err(|e| {
            fetching_err(format!("Failed to re-fetch tenant DEK row: {e}"))
        })?
        .ok_or_else(|| {
            fetching_err(format!("Tenant not found: {tid}")).with_exp_true()
        })?;

    unwrap_dek(&wrapped_existing, kek, aad)
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_support::setup_temp_db, types::naive_timestamp_to_text};

    #[tokio::test]
    async fn get_or_provision_dek_round_trips_through_sqlite(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let fetching = EncryptionKeyFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let kek = [7u8; 32];

        // Seed the well-known system tenant row (id = Uuid::nil()).
        {
            let conn = &mut db.provider.get_pool().get().unwrap();
            diesel::insert_into(tenant::table)
                .values((
                    tenant::id.eq(uuid_to_text(&SYSTEM_TENANT_ID)),
                    tenant::name.eq("System"),
                    tenant::created.eq(naive_timestamp_to_text(
                        &chrono::Utc::now().naive_utc(),
                    )),
                    tenant::kek_version.eq(1),
                ))
                .execute(conn)
                .unwrap();
        }

        // First call provisions a new DEK and persists the wrapped form
        let first_dek = fetching.get_or_provision_dek(None, &kek).await?;

        let stored_wrapped = tenant_dsl::tenant
            .filter(tenant::id.eq(uuid_to_text(&SYSTEM_TENANT_ID)))
            .select(tenant::encrypted_dek)
            .first::<Option<String>>(&mut db.provider.get_pool().get().unwrap())
            .unwrap();
        assert!(stored_wrapped.is_some());

        // Second call unwraps the persisted DEK -- must return the same key
        let second_dek = fetching.get_or_provision_dek(None, &kek).await?;
        assert_eq!(first_dek, second_dek);

        Ok(())
    }

    #[tokio::test]
    async fn get_or_provision_dek_self_heals_missing_system_tenant(
    ) -> Result<(), MappedErrors> {
        // A fresh database has no system tenant row (no migration seeds it).
        // The system DEK fetch must create the sentinel row on first use
        // instead of failing with "Tenant not found".
        let db = setup_temp_db();
        let fetching = EncryptionKeyFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let kek = [9u8; 32];

        // No seeding here on purpose.
        let first_dek = fetching.get_or_provision_dek(None, &kek).await?;

        // The sentinel row now exists with a persisted wrapped DEK.
        let stored_wrapped = tenant_dsl::tenant
            .filter(tenant::id.eq(uuid_to_text(&SYSTEM_TENANT_ID)))
            .select(tenant::encrypted_dek)
            .first::<Option<String>>(&mut db.provider.get_pool().get().unwrap())
            .unwrap();
        assert!(stored_wrapped.is_some());

        // The DEK is stable across calls once provisioned.
        let second_dek = fetching.get_or_provision_dek(None, &kek).await?;
        assert_eq!(first_dek, second_dek);

        Ok(())
    }

    #[tokio::test]
    async fn provision_and_persist_dek_does_not_overwrite_existing(
    ) -> Result<(), MappedErrors> {
        // A second provisioning attempt (e.g. a concurrent caller) must not
        // rekey an already-provisioned DEK -- it re-reads and returns the
        // persisted one.
        let db = setup_temp_db();
        let kek = [5u8; 32];
        let tid_text = uuid_to_text(&SYSTEM_TENANT_ID);

        let conn = &mut db.provider.get_pool().get().unwrap();
        diesel::insert_into(tenant::table)
            .values((
                tenant::id.eq(&tid_text),
                tenant::name.eq(SYSTEM_TENANT_NAME),
                tenant::created.eq(naive_timestamp_to_text(
                    &chrono::Utc::now().naive_utc(),
                )),
                tenant::kek_version.eq(1),
            ))
            .execute(conn)
            .unwrap();

        let first =
            provision_and_persist_dek(conn, &tid_text, SYSTEM_TENANT_ID, &kek)?;
        let second =
            provision_and_persist_dek(conn, &tid_text, SYSTEM_TENANT_ID, &kek)?;

        assert_eq!(first, second);

        Ok(())
    }

    #[tokio::test]
    async fn get_or_provision_dek_errors_for_missing_real_tenant(
    ) -> Result<(), MappedErrors> {
        // A missing *real* tenant is a genuine error -- only the system
        // sentinel row self-heals.
        let db = setup_temp_db();
        let fetching = EncryptionKeyFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let kek = [3u8; 32];

        let result = fetching
            .get_or_provision_dek(Some(Uuid::new_v4()), &kek)
            .await;

        assert!(result.is_err());

        Ok(())
    }
}
