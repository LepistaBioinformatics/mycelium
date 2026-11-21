use crate::functions::try_to_resolve_database_url;

use clap::Parser;
use myc_core::{
    domain::{
        dtos::{
            federation::{FederationPublicJwk, InstanceSelfDescription},
            instance_settings::FEDERATION_INSTANCE_ID_KEY,
            profile_signature::SigningKeyVersion,
        },
        entities::InstanceSettingsFetching,
    },
    models::{CoreConfig, FederationConfig},
    use_cases::federation::describe_instance,
};
use myc_diesel::repositories::{
    DieselDbPoolProvider, DieselDbPoolProviderParameters, SqlAppModule,
};
use myc_http_tools::models::{KeyDerivationPurpose, VersionedEd25519Signer};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{execution_err, MappedErrors},
};
use shaku::HasComponent;
use std::{env::var, path::PathBuf, process::exit};
use uuid::Uuid;

#[derive(Parser, Debug)]
pub(crate) struct Arguments {
    #[clap(subcommand)]
    pub cmd: Commands,
}

#[derive(Parser, Debug)]
pub(crate) enum Commands {
    /// Print the self-description this instance serves at
    /// `/.well-known/mycelium-instance`, from `SETTINGS_PATH`
    ///
    /// Reads the instance id from the database (`DATABASE_URL`); never calls
    /// a peer and never prints private material.
    Describe,
}

/// Print the self-description as JSON, or exit non-zero with the reason
pub(crate) async fn describe_cmd() {
    match describe().await {
        Ok(description) => println!(
            "{}",
            serde_json::to_string_pretty(&description)
                .expect("a self-description always serializes")
        ),
        Err(err) => {
            tracing::error!("federation describe failed: {err}");
            exit(1);
        }
    }
}

async fn describe() -> Result<InstanceSelfDescription, MappedErrors> {
    let settings_path = PathBuf::from(
        var("SETTINGS_PATH")
            .map_err(|_| execution_err("SETTINGS_PATH env var is required"))?,
    );

    let core = CoreConfig::from_default_config_file(settings_path.clone())?;

    // Same loader, so the same message, as the gateway boot check.
    let config = FederationConfig::from_default_config_file(
        settings_path,
        &core.account_life_cycle,
    )?
    .ok_or_else(|| {
        execution_err("[core.federation] is absent from the configuration")
    })?;

    let secrets = core.account_life_cycle.hmac_signing_keys().await?;
    let instance_id = read_instance_id().await?;

    build_description(instance_id, &config, &secrets)
}

/// The persisted instance id; the gateway mints it on its first boot
async fn read_instance_id() -> Result<Uuid, MappedErrors> {
    let database_url = try_to_resolve_database_url();
    let module = SqlAppModule::builder()
        .with_component_parameters::<DieselDbPoolProvider>(
            DieselDbPoolProviderParameters {
                pool: DieselDbPoolProvider::new(&database_url),
            },
        )
        .build();
    let repo: &dyn InstanceSettingsFetching = module.resolve_ref();

    let FetchResponseKind::Found(setting) =
        repo.get(FEDERATION_INSTANCE_ID_KEY.to_string()).await?
    else {
        return execution_err(
            "no instance id yet: boot the gateway once with \
             [core.federation] to mint it",
        )
        .as_error();
    };

    setting
        .value
        .as_str()
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| {
            execution_err(format!(
                "instance setting {FEDERATION_INSTANCE_ID_KEY} does not hold \
                 a UUID"
            ))
        })
}

/// The document the gateway serves: one public key per HMAC version except
/// the reserved version `0`
fn build_description(
    instance_id: Uuid,
    config: &FederationConfig,
    secrets: &[(u32, Vec<u8>)],
) -> Result<InstanceSelfDescription, MappedErrors> {
    let mut keys = Vec::with_capacity(secrets.len());
    for (raw_version, secret) in secrets {
        let Ok(version) = SigningKeyVersion::try_from(*raw_version) else {
            continue;
        };

        let signer = VersionedEd25519Signer::derive_from_secret(
            version,
            secret,
            KeyDerivationPurpose::Federation,
        )?;

        keys.push(
            serde_json::from_value::<FederationPublicJwk>(
                signer.public_jwk().to_json(),
            )
            .map_err(|err| execution_err(err.to_string()))?,
        );
    }

    Ok(describe_instance(
        instance_id,
        config,
        env!("CARGO_PKG_VERSION"),
        keys,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cli;

    fn config() -> FederationConfig {
        serde_json::from_value(serde_json::json!({
            "displayName": "Instance 01",
            "baseUrl": "https://myc-01.example.com",
        }))
        .unwrap()
    }

    fn secrets(versions: &[u32]) -> Vec<(u32, Vec<u8>)> {
        versions
            .iter()
            .map(|version| {
                (*version, format!("secret-v{version}").into_bytes())
            })
            .collect()
    }

    #[test]
    fn parses_federation_describe() {
        let cli =
            Cli::try_parse_from(["myc-cli", "federation", "describe"]).unwrap();

        assert!(matches!(
            cli,
            Cli::Federation(Arguments {
                cmd: Commands::Describe
            })
        ));
    }

    #[test]
    fn prints_one_public_key_per_version_and_nothing_private() {
        let instance_id = Uuid::new_v4();
        let description =
            build_description(instance_id, &config(), &secrets(&[0, 1, 2]))
                .unwrap();
        let document = serde_json::to_value(&description).unwrap();

        assert_eq!(document["instanceId"], instance_id.to_string());
        assert_eq!(document["baseUrl"], "https://myc-01.example.com");
        assert_eq!(document["displayName"], "Instance 01");

        let kids: Vec<_> = document["keys"]
            .as_array()
            .unwrap()
            .iter()
            .map(|key| key["kid"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(kids, ["1", "2"]);

        for key in document["keys"].as_array().unwrap() {
            assert!(key.get("d").is_none(), "{key}");
        }
        assert!(!document.to_string().contains("secret-v"));
    }
}
