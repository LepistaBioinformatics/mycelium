use crate::functions::try_to_resolve_database_url;

use clap::Parser;
use myc_core::models::CoreConfig;
use myc_diesel::{migration::rotate_kek, repositories::DieselDbPoolProvider};
use std::{env::var, path::PathBuf};

/// Environment variable the operator sets to the **old** `token_secret`
/// while running `rotate-kek`. The current/new `token_secret` stays in
/// the config file.
const OLD_TOKEN_SECRET_ENV: &str = "MYC_OLD_TOKEN_SECRET";

#[derive(Parser, Debug)]
pub(crate) struct Arguments {
    #[clap(subcommand)]
    pub cmd: Commands,
}

#[derive(Parser, Debug)]
pub(crate) enum Commands {
    /// Rewrap every tenant's DEK from an old KEK (derived from the old
    /// `token_secret`) to the new KEK currently configured.
    ///
    /// The old `token_secret` value is provided via the
    /// `MYC_OLD_TOKEN_SECRET` env var. User-data ciphertexts are never
    /// touched — only the per-tenant DEK wrapping is rewrapped.
    RotateKek(RotateKekArguments),
}

#[derive(Parser, Debug)]
pub(crate) struct RotateKekArguments {
    /// KEK generation currently stored on each tenant row (the `from` side
    /// of the rotation).
    #[clap(long, value_name = "N")]
    pub from_version: u32,

    /// KEK generation to persist after rewrap (the `to` side of the
    /// rotation).
    #[clap(long, value_name = "M")]
    pub to_version: u32,

    /// Scan and report without writing any changes.
    #[clap(long)]
    pub dry_run: bool,
}

#[tracing::instrument(name = "rotate_kek_cmd", skip_all)]
pub(crate) async fn rotate_kek_cmd(args: RotateKekArguments) {
    let settings_path = match var("SETTINGS_PATH") {
        Ok(p) => p,
        Err(_) => {
            tracing::error!("SETTINGS_PATH env var is required for rotate-kek",);
            return;
        }
    };

    let old_token_secret = match var(OLD_TOKEN_SECRET_ENV) {
        Ok(v) => v,
        Err(_) => {
            tracing::error!(
                "{OLD_TOKEN_SECRET_ENV} env var is required for rotate-kek",
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

    let new_config = core_config.account_life_cycle.clone();
    let old_config = new_config.with_token_secret_override(old_token_secret);

    let database_url = try_to_resolve_database_url();
    let pool = DieselDbPoolProvider::new(&database_url);

    if args.dry_run {
        tracing::info!("Running in dry-run mode — no changes will be written");
    }

    match rotate_kek(
        &pool,
        args.from_version,
        args.to_version,
        &old_config,
        &new_config,
        args.dry_run,
    )
    .await
    {
        Err(err) => tracing::error!("rotate-kek failed: {err}"),
        Ok(report) => {
            tracing::info!(
                "rotate-kek {}",
                if report.summary.dry_run {
                    "(dry-run)"
                } else {
                    "complete"
                },
            );
            tracing::info!("  From version:    {}", report.from_version,);
            tracing::info!("  To version:      {}", report.to_version);
            tracing::info!("  Tenants scanned: {}", report.summary.scanned,);
            tracing::info!("  Migrated:        {}", report.summary.migrated,);
            tracing::info!(
                "  Already done:    {}",
                report.summary.already_done,
            );
            tracing::info!("  Skipped:         {}", report.summary.skipped);
        }
    }
}
