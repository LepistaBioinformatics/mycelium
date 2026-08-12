use super::tenant_iteration::{
    acquire_conn, load_tenants, RowOutcome, Summary,
};
use crate::{
    models::{config::DbPool, tenant::Tenant as TenantModel},
    schema::tenant::{self as tenant_model, dsl as tenant_dsl},
};

use diesel::prelude::*;
use myc_core::{
    domain::utils::{unwrap_dek, wrap_dek},
    models::AccountLifeCycle,
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};

/// Summary of a `rotate-kek` pass.
///
/// User-data ciphertexts (`v2:` prefixed fields under `user.mfa`,
/// `tenant.meta`, `webhook.secret`) are **not** touched — only the
/// per-tenant DEK wrapping is rewrapped under the new KEK.
pub struct RotateKekReport {
    pub from_version: i32,
    pub to_version: i32,
    pub summary: Summary,
}

/// Rewrap every eligible tenant's `encrypted_dek` column from an old KEK
/// to a new KEK without touching any user-data ciphertext.
///
/// Eligible rows are those whose `kek_version == from_version` AND whose
/// `encrypted_dek` is set. Rows whose `kek_version == to_version` already
/// are reported as `RowOutcome::AlreadyDone` so the command is idempotent
/// across re-runs. Rows in any other state are `Skipped` (they belong to a
/// different rotation window and must be handled by the matching pair of
/// configs).
///
/// # Parameters
///
/// - `old_config` resolves the **old** `token_secret` (used to unwrap the
///   stored DEK).
/// - `new_config` resolves the **new** `token_secret` (used to wrap the
///   DEK for persistence).
/// - `dry_run` short-circuits the UPDATE so operators can preview counts.
#[tracing::instrument(name = "rotate_kek", skip_all)]
pub async fn rotate_kek(
    pool: &DbPool,
    from_version: u32,
    to_version: u32,
    old_config: &AccountLifeCycle,
    new_config: &AccountLifeCycle,
    dry_run: bool,
) -> Result<RotateKekReport, MappedErrors> {
    if from_version == to_version {
        return execution_err("rotate_kek requires from_version != to_version")
            .as_error();
    }

    let from_version_i32 = i32::try_from(from_version).map_err(|_| {
        execution_err(format!("from_version {from_version} exceeds i32 range",))
    })?;

    let to_version_i32 = i32::try_from(to_version).map_err(|_| {
        execution_err(format!("to_version {to_version} exceeds i32 range",))
    })?;

    let old_kek = old_config.derive_kek_bytes().await?;
    let new_kek = new_config.derive_kek_bytes().await?;

    let conn = &mut acquire_conn(pool)?;

    let tenants = load_tenants(conn, None)?;
    let mut summary = Summary::new(dry_run);

    for tenant in &tenants {
        let outcome = rewrap_one(
            conn,
            tenant,
            from_version_i32,
            to_version_i32,
            &old_kek,
            &new_kek,
            dry_run,
        )?;
        summary.record(outcome);
    }

    Ok(RotateKekReport {
        from_version: from_version_i32,
        to_version: to_version_i32,
        summary,
    })
}

fn rewrap_one(
    conn: &mut PgConnection,
    tenant: &TenantModel,
    from_version: i32,
    to_version: i32,
    old_kek: &[u8; 32],
    new_kek: &[u8; 32],
    dry_run: bool,
) -> Result<RowOutcome, MappedErrors> {
    if tenant.kek_version == to_version {
        return Ok(RowOutcome::AlreadyDone);
    }

    if tenant.kek_version != from_version {
        return Ok(RowOutcome::Skipped);
    }

    let Some(wrapped) = tenant.encrypted_dek.as_deref() else {
        return Ok(RowOutcome::Skipped);
    };

    let tid = tenant.id;
    let dek = unwrap_dek(wrapped, old_kek, tid.as_bytes())?;
    let rewrapped = wrap_dek(&dek, new_kek, tid.as_bytes())?;

    if dry_run {
        return Ok(RowOutcome::Migrated);
    }

    diesel::update(tenant_dsl::tenant.filter(tenant_model::id.eq(tid)))
        .set((
            tenant_model::encrypted_dek.eq(&rewrapped),
            tenant_model::kek_version.eq(to_version),
        ))
        .execute(conn)
        .map_err(|e| {
            execution_err(format!(
                "Failed to persist rewrapped DEK for tenant {tid}: {e}",
            ))
        })?;

    Ok(RowOutcome::Migrated)
}
