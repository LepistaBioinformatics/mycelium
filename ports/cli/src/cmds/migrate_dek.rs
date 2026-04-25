use crate::functions::try_to_resolve_database_url;

use clap::Parser;
use myc_core::models::CoreConfig;
use myc_diesel::{migration::migrate_dek, repositories::DieselDbPoolProvider};
use std::{env::var, path::PathBuf};
use uuid::Uuid;

#[derive(Parser, Debug)]
pub(crate) struct Arguments {
    #[clap(subcommand)]
    pub cmd: Commands,
}

#[derive(Parser, Debug)]
pub(crate) enum Commands {
    /// Re-encrypt all v1 ciphertexts to v2 envelope encryption format.
    MigrateDek(MigrateDekArguments),
}

#[derive(Parser, Debug)]
pub(crate) struct MigrateDekArguments {
    /// Scan and report without writing any changes.
    #[clap(long)]
    pub dry_run: bool,

    /// Restrict migration to a single tenant by UUID.
    #[clap(long, value_name = "UUID")]
    pub tenant_id: Option<Uuid>,
}

#[tracing::instrument(name = "migrate_dek_cmd", skip_all)]
pub(crate) async fn migrate_dek_cmd(args: MigrateDekArguments) {
    let settings_path = match var("SETTINGS_PATH") {
        Ok(p) => p,
        Err(_) => {
            tracing::error!(
                "SETTINGS_PATH env var is required for migrate-dek"
            );
            return;
        }
    };

    let core_config = match CoreConfig::from_default_config_file(PathBuf::from(
        &settings_path,
    )) {
        Ok(c) => c,
        Err(err) => {
            tracing::error!("Failed to load core config: {err}");
            return;
        }
    };

    let database_url = try_to_resolve_database_url();
    let pool = DieselDbPoolProvider::new(&database_url);

    if args.dry_run {
        tracing::info!("Running in dry-run mode — no changes will be written");
    }

    match migrate_dek(
        &pool,
        &core_config.account_life_cycle,
        args.tenant_id,
        args.dry_run,
    )
    .await
    {
        Err(err) => tracing::error!("Migration failed: {err}"),
        Ok(report) => {
            tracing::info!(
                "Migration {}",
                if report.dry_run {
                    "(dry-run)"
                } else {
                    "complete"
                }
            );
            tracing::info!(
                "  Tenants scanned:          {}",
                report.tenants_scanned
            );
            tracing::info!(
                "  TOTP fields migrated:     {}",
                report.totp_fields_migrated
            );
            tracing::info!(
                "  Telegram fields migrated: {}",
                report.telegram_fields_migrated
            );
            tracing::info!(
                "  Webhook secrets migrated: {}",
                report.webhook_secrets_migrated
            );
        }
    }
}
