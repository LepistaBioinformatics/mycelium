use super::tenant_iteration::{acquire_conn, load_tenants};
use crate::{
    models::{
        config::DbPool, tenant::Tenant as TenantModel, user::User as UserModel,
    },
    schema::{
        tenant::{self as tenant_model, dsl as tenant_dsl},
        user::{self as user_model, dsl as user_dsl},
        webhook::{self as webhook_model, dsl as webhook_dsl},
    },
};

use diesel::prelude::*;
use myc_core::{
    domain::{
        dtos::tenant::TenantMetaKey,
        utils::{
            build_aad, decrypt_string, encrypt_with_dek, generate_dek,
            unwrap_dek, wrap_dek, AAD_FIELD_HTTP_SECRET,
            AAD_FIELD_TELEGRAM_BOT_TOKEN, AAD_FIELD_TELEGRAM_WEBHOOK_SECRET,
            AAD_FIELD_TOTP_SECRET, SYSTEM_TENANT_ID,
        },
    },
    models::AccountLifeCycle,
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Summary of a dry-run or live migration pass.
pub struct MigrateDekReport {
    pub tenants_scanned: usize,
    pub totp_fields_migrated: usize,
    pub telegram_fields_migrated: usize,
    pub webhook_secrets_migrated: usize,
    pub dry_run: bool,
}

/// Re-encrypt all v1 ciphertexts to v2 across TOTP, Telegram meta, and
/// webhook secrets.
///
/// Set `dry_run = true` to scan and count without writing anything.
/// Set `tenant_id = Some(uuid)` to restrict migration to one tenant.
#[tracing::instrument(name = "migrate_dek", skip_all)]
pub async fn migrate_dek(
    pool: &DbPool,
    life_cycle: &AccountLifeCycle,
    tenant_id: Option<Uuid>,
    dry_run: bool,
) -> Result<MigrateDekReport, MappedErrors> {
    let kek = life_cycle.derive_kek_bytes().await?;
    let conn = &mut acquire_conn(pool)?;

    let mut report = MigrateDekReport {
        tenants_scanned: 0,
        totp_fields_migrated: 0,
        telegram_fields_migrated: 0,
        webhook_secrets_migrated: 0,
        dry_run,
    };

    // ? -----------------------------------------------------------------------
    // ? Load tenants to process
    // ? -----------------------------------------------------------------------

    let tenants = load_tenants(conn, tenant_id)?;

    report.tenants_scanned = tenants.len();

    // ? -----------------------------------------------------------------------
    // ? For each tenant: migrate TOTP secrets and Telegram meta
    // ? -----------------------------------------------------------------------

    for tenant in &tenants {
        let dek = get_or_provision_dek(conn, tenant, &kek, dry_run)?;

        report.totp_fields_migrated +=
            migrate_totp_for_tenant(conn, life_cycle, tenant.id, &dek, dry_run)
                .await?;

        report.telegram_fields_migrated += migrate_telegram_meta_for_tenant(
            conn, life_cycle, tenant, &dek, dry_run,
        )
        .await?;
    }

    // ? -----------------------------------------------------------------------
    // ? Migrate system webhook secrets (system DEK / UUID::nil)
    // ? -----------------------------------------------------------------------

    if tenant_id.is_none() {
        let system_tenant = tenant_dsl::tenant
            .filter(tenant_model::id.eq(SYSTEM_TENANT_ID))
            .select(TenantModel::as_select())
            .first::<TenantModel>(conn)
            .optional()
            .map_err(|e| {
                execution_err(format!("Failed to load system tenant: {e}"))
            })?;

        if let Some(ref st) = system_tenant {
            let system_dek = get_or_provision_dek(conn, st, &kek, dry_run)?;
            report.webhook_secrets_migrated +=
                migrate_webhook_secrets(conn, life_cycle, &system_dek, dry_run)
                    .await?;
        }
    }

    Ok(report)
}

// ---------------------------------------------------------------------------
// DEK provisioning
// ---------------------------------------------------------------------------

fn get_or_provision_dek(
    conn: &mut PgConnection,
    tenant: &TenantModel,
    kek: &[u8; 32],
    dry_run: bool,
) -> Result<[u8; 32], MappedErrors> {
    let tid = tenant.id;

    if let Some(wrapped) = &tenant.encrypted_dek {
        return unwrap_dek(wrapped, kek, tid.as_bytes());
    }

    let dek = generate_dek()?;
    if dry_run {
        return Ok(dek);
    }

    let wrapped = wrap_dek(&dek, kek, tid.as_bytes())?;
    diesel::update(tenant_dsl::tenant.filter(tenant_model::id.eq(tid)))
        .set(tenant_model::encrypted_dek.eq(&wrapped))
        .execute(conn)
        .map_err(|e| {
            execution_err(format!(
                "Failed to persist DEK for tenant {tid}: {e}"
            ))
        })?;

    Ok(dek)
}

// ---------------------------------------------------------------------------
// TOTP migration
// ---------------------------------------------------------------------------

async fn migrate_totp_for_tenant(
    conn: &mut PgConnection,
    life_cycle: &AccountLifeCycle,
    tenant_id: Uuid,
    dek: &[u8; 32],
    dry_run: bool,
) -> Result<usize, MappedErrors> {
    use crate::schema::account::{self as account_model, dsl as account_dsl};

    // Load all account IDs for this tenant
    let account_ids: Vec<Uuid> = account_dsl::account
        .filter(account_model::tenant_id.eq(tenant_id))
        .select(account_model::id)
        .load::<Uuid>(conn)
        .map_err(|e| execution_err(format!("Failed to load accounts: {e}")))?;

    // Load all users for those accounts that have mfa set
    let users: Vec<UserModel> = user_dsl::user
        .filter(user_model::account_id.eq_any(&account_ids))
        .filter(user_model::mfa.is_not_null())
        .select(UserModel::as_select())
        .load::<UserModel>(conn)
        .map_err(|e| execution_err(format!("Failed to load users: {e}")))?;

    let mut migrated = 0usize;

    let aad = build_aad(Some(tenant_id), AAD_FIELD_TOTP_SECRET);

    for user in users {
        let Some(mfa_json) = &user.mfa else {
            continue;
        };

        let updated = migrate_totp_json(mfa_json, life_cycle, dek, &aad).await;
        let Some(updated_json) = updated else {
            continue;
        };

        migrated += 1;
        if dry_run {
            continue;
        }

        diesel::update(user_dsl::user.filter(user_model::id.eq(user.id)))
            .set(user_model::mfa.eq(updated_json))
            .execute(conn)
            .map_err(|e| {
                execution_err(format!(
                    "Failed to update MFA for user {}: {e}",
                    user.id
                ))
            })?;
    }

    Ok(migrated)
}

/// Returns Some(updated_json) if the secret was migrated, None otherwise.
async fn migrate_totp_json(
    mfa_json: &JsonValue,
    life_cycle: &AccountLifeCycle,
    dek: &[u8; 32],
    aad: &[u8],
) -> Option<JsonValue> {
    // The MFA JSON has shape: {"totp": {"enabled": {..., "secret": "..."}} }
    // or {"totp": "disabled"} / {"totp": "unknown"}
    let secret = mfa_json
        .get("totp")
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.get("secret"))
        .and_then(|v| v.as_str())?;

    if secret.starts_with("v2:") {
        return None;
    }

    let plaintext = decrypt_string(secret, life_cycle).await.ok()?;
    let re_encrypted = encrypt_with_dek(&plaintext, dek, aad).ok()?;

    let mut updated = mfa_json.clone();
    *updated
        .pointer_mut("/totp/enabled/secret")
        .expect("path was valid since we read from it") =
        JsonValue::String(re_encrypted);

    Some(updated)
}

// ---------------------------------------------------------------------------
// Telegram meta migration
// ---------------------------------------------------------------------------

async fn migrate_telegram_meta_for_tenant(
    conn: &mut PgConnection,
    life_cycle: &AccountLifeCycle,
    tenant: &TenantModel,
    dek: &[u8; 32],
    dry_run: bool,
) -> Result<usize, MappedErrors> {
    let Some(meta_json) = &tenant.meta else {
        return Ok(0);
    };

    let tenant_id = tenant.id;
    let mut meta_map: std::collections::HashMap<String, String> =
        serde_json::from_value(meta_json.clone()).unwrap_or_default();

    let mut migrated = 0usize;

    let bot_key = TenantMetaKey::TelegramBotToken.to_string();
    let webhook_key = TenantMetaKey::TelegramWebhookSecret.to_string();

    let aad_bot = build_aad(Some(tenant_id), AAD_FIELD_TELEGRAM_BOT_TOKEN);
    let aad_webhook =
        build_aad(Some(tenant_id), AAD_FIELD_TELEGRAM_WEBHOOK_SECRET);

    migrated +=
        migrate_meta_field(&mut meta_map, &bot_key, life_cycle, dek, &aad_bot)
            .await;
    migrated += migrate_meta_field(
        &mut meta_map,
        &webhook_key,
        life_cycle,
        dek,
        &aad_webhook,
    )
    .await;

    if migrated == 0 || dry_run {
        return Ok(migrated);
    }

    let updated_meta = serde_json::to_value(&meta_map)
        .map_err(|e| execution_err(format!("Failed to serialize meta: {e}")))?;

    diesel::update(tenant_dsl::tenant.filter(tenant_model::id.eq(tenant_id)))
        .set(tenant_model::meta.eq(updated_meta))
        .execute(conn)
        .map_err(|e| {
            execution_err(format!(
                "Failed to update meta for tenant {tenant_id}: {e}"
            ))
        })?;

    Ok(migrated)
}

// ---------------------------------------------------------------------------
// Telegram meta field helper
// ---------------------------------------------------------------------------

async fn migrate_meta_field(
    meta_map: &mut std::collections::HashMap<String, String>,
    key: &str,
    life_cycle: &AccountLifeCycle,
    dek: &[u8; 32],
    aad: &[u8],
) -> usize {
    let Some(val) = meta_map.get(key) else {
        return 0;
    };
    if val.starts_with("v2:") {
        return 0;
    }
    let Ok(plain) = decrypt_string(val, life_cycle).await else {
        return 0;
    };
    let Ok(enc) = encrypt_with_dek(&plain, dek, aad) else {
        return 0;
    };
    meta_map.insert(key.to_owned(), enc);
    1
}

// ---------------------------------------------------------------------------
// Webhook secret migration (system DEK)
// ---------------------------------------------------------------------------

async fn migrate_webhook_secrets(
    conn: &mut PgConnection,
    life_cycle: &AccountLifeCycle,
    system_dek: &[u8; 32],
    dry_run: bool,
) -> Result<usize, MappedErrors> {
    let aad = build_aad(None, AAD_FIELD_HTTP_SECRET);

    let webhooks: Vec<(Uuid, Option<JsonValue>)> = webhook_dsl::webhook
        .filter(webhook_model::secret.is_not_null())
        .select((webhook_model::id, webhook_model::secret))
        .load::<(Uuid, Option<JsonValue>)>(conn)
        .map_err(|e| execution_err(format!("Failed to load webhooks: {e}")))?;

    let mut migrated = 0usize;

    for (hook_id, secret_json) in webhooks {
        let Some(json) = secret_json else {
            continue;
        };

        let Some(updated_json) =
            migrate_http_secret_json(&json, life_cycle, system_dek, &aad).await
        else {
            continue;
        };

        migrated += 1;
        if dry_run {
            continue;
        }

        diesel::update(
            webhook_dsl::webhook.filter(webhook_model::id.eq(hook_id)),
        )
        .set(webhook_model::secret.eq(updated_json))
        .execute(conn)
        .map_err(|e| {
            execution_err(format!(
                "Failed to update secret for webhook {hook_id}: {e}"
            ))
        })?;
    }

    Ok(migrated)
}

/// Returns Some(updated_json) if the token was migrated, None otherwise.
async fn migrate_http_secret_json(
    json: &JsonValue,
    life_cycle: &AccountLifeCycle,
    dek: &[u8; 32],
    aad: &[u8],
) -> Option<JsonValue> {
    // HttpSecret is stored as-is in JSONB. Extract the token field.
    // Shape: {"authorizationHeader": {..., "token": "..."}} or
    //        {"queryParameter": {..., "token": "..."}}
    let (token, path) = if let Some(t) = json
        .get("authorizationHeader")
        .and_then(|v| v.get("token"))
        .and_then(|v| v.as_str())
    {
        (t, "/authorizationHeader/token")
    } else if let Some(t) = json
        .get("queryParameter")
        .and_then(|v| v.get("token"))
        .and_then(|v| v.as_str())
    {
        (t, "/queryParameter/token")
    } else {
        return None;
    };

    if token.starts_with("v2:") {
        return None;
    }

    let plaintext = decrypt_string(token, life_cycle).await.ok()?;

    // Attempt direct DEK decryption in case the v1 ciphertext was already
    // tagged for this field (should not happen, but be safe).
    let re_encrypted = encrypt_with_dek(&plaintext, dek, aad).ok()?;

    let mut updated = json.clone();
    *updated.pointer_mut(path)? = JsonValue::String(re_encrypted);

    Some(updated)
}
