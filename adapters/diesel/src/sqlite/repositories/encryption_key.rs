use crate::sqlite::{
    config::SqliteDbPoolProvider,
    models::tenant::Tenant as TenantModel,
    schema::tenant::{self, dsl as tenant_dsl},
    types::uuid_to_text,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes,
    entities::EncryptionKeyFetching,
    utils::{generate_dek, unwrap_dek, wrap_dek, SYSTEM_TENANT_ID},
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
            })?
            .ok_or_else(|| {
                fetching_err(format!("Tenant not found: {tid}")).with_exp_true()
            })?;

        if let Some(wrapped) = record.encrypted_dek {
            let aad = tid.as_bytes();
            return unwrap_dek(&wrapped, kek, aad);
        }

        provision_and_persist_dek(conn, &tid_text, tid, kek)
    }
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

    diesel::update(tenant_dsl::tenant.filter(tenant::id.eq(tid_text)))
        .set(tenant::encrypted_dek.eq(&wrapped))
        .execute(conn)
        .map_err(|e| {
            updating_err(format!("Failed to persist DEK for tenant {tid}: {e}"))
        })?;

    Ok(dek)
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::{
        test_support::setup_temp_db, types::naive_timestamp_to_text,
    };

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
}
