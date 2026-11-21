use crate::federation::PreparedFederation;

use actix_web::{web, HttpResponse};
use myc_core::{
    domain::{
        dtos::federation::{FederationPublicJwk, InstanceSelfDescription},
        entities::InstanceSettingsFetching,
    },
    use_cases::federation::{describe_instance, is_federation_enabled},
};

/// Where the instance self-description is served (FED-03)
pub(crate) const INSTANCE_SELF_DESCRIPTION_PATH: &str =
    "/.well-known/mycelium-instance";

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.route(
        INSTANCE_SELF_DESCRIPTION_PATH,
        web::get().to(get_instance_self_description),
    );
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// The public self-description of this instance
///
/// Instance id, display name, canonical base URL, gateway version, supported
/// assertion kinds and every valid federation public key. Public and
/// unauthenticated by design, like `/app-config`; never private material.
/// `404` when `[core.federation]` is absent or federation is switched off.
///
#[tracing::instrument(name = "get_instance_self_description", skip_all)]
pub async fn get_instance_self_description(
    prepared: web::Data<Option<PreparedFederation>>,
    instance_settings: web::Data<dyn InstanceSettingsFetching>,
) -> HttpResponse {
    let Some(prepared) = prepared.get_ref() else {
        return HttpResponse::NotFound().finish();
    };

    match is_federation_enabled(Box::new(instance_settings.get_ref())).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::NotFound().finish(),
        Err(err) => {
            tracing::error!(%err, "federation switch unreadable");
            return HttpResponse::InternalServerError().finish();
        }
    }

    HttpResponse::Ok().json(self_description(prepared))
}

fn self_description(prepared: &PreparedFederation) -> InstanceSelfDescription {
    let keys = prepared
        .signing
        .public_keys
        .iter()
        .map(|key| {
            serde_json::from_value::<FederationPublicJwk>(key.to_json())
                .expect("a published Ed25519 JWK maps to FederationPublicJwk")
        })
        .collect();

    describe_instance(
        prepared.instance_id,
        &prepared.config,
        env!("CARGO_PKG_VERSION"),
        keys,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::FederationSigning;

    use actix_web::{dev::ServiceResponse, http::StatusCode, test, App};
    use async_trait::async_trait;
    use chrono::Local;
    use myc_core::{
        domain::dtos::{
            instance_settings::InstanceSetting,
            profile_signature::SigningKeyVersion,
        },
        models::FederationConfig,
    };
    use myc_http_tools::models::{
        KeyDerivationPurpose, VersionedEd25519Signer,
    };
    use mycelium_base::{
        entities::FetchResponseKind, utils::errors::MappedErrors,
    };
    use serde_json::Value;
    use std::sync::Arc;
    use uuid::Uuid;

    struct Switch(Option<bool>);

    #[async_trait]
    impl InstanceSettingsFetching for Switch {
        async fn get(
            &self,
            key: String,
        ) -> Result<FetchResponseKind<InstanceSetting, ()>, MappedErrors>
        {
            Ok(match self.0 {
                None => FetchResponseKind::NotFound(None),
                Some(on) => FetchResponseKind::Found(InstanceSetting {
                    key,
                    value: serde_json::json!(on),
                    created_by: None,
                    updated_by: None,
                    created: Local::now(),
                    updated: None,
                }),
            })
        }
    }

    fn prepared() -> PreparedFederation {
        let signer = VersionedEd25519Signer::derive_from_secret(
            SigningKeyVersion::try_from(1).unwrap(),
            b"test-only-secret",
            KeyDerivationPurpose::Federation,
        )
        .unwrap();

        PreparedFederation {
            config: serde_json::from_value::<FederationConfig>(
                serde_json::json!({
                    "displayName": "Instance 01",
                    "baseUrl": "https://myc-01.example.com",
                }),
            )
            .unwrap(),
            instance_id: Uuid::new_v4(),
            signing: FederationSigning {
                public_keys: vec![signer.public_jwk().clone()],
                primary: Arc::new(signer),
            },
        }
    }

    async fn call(
        prepared: Option<PreparedFederation>,
        switch: Option<bool>,
    ) -> ServiceResponse {
        let instance_settings: Arc<dyn InstanceSettingsFetching> =
            Arc::new(Switch(switch));
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(prepared))
                .app_data(web::Data::from(instance_settings))
                .configure(configure),
        )
        .await;
        let request =
            test::TestRequest::get().uri(INSTANCE_SELF_DESCRIPTION_PATH);

        test::call_service(&app, request.to_request()).await
    }

    #[actix_web::test]
    async fn serves_the_document_when_enabled_without_credentials() {
        let prepared = prepared();
        let response = call(Some(prepared.clone()), Some(true)).await;

        assert_eq!(response.status(), StatusCode::OK);

        let document: Value = test::read_body_json(response).await;
        assert_eq!(document["instanceId"], prepared.instance_id.to_string());
        assert_eq!(document["baseUrl"], "https://myc-01.example.com");
        assert_eq!(document["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(document["keys"][0]["kid"], "1");
        assert_eq!(
            document["keys"][0]["x"],
            prepared.signing.public_keys[0].x()
        );
        assert!(document["keys"][0].get("d").is_none());
    }

    #[actix_web::test]
    async fn not_found_when_switched_off() {
        for switch in [None, Some(false)] {
            let response = call(Some(prepared()), switch).await;

            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
    }

    #[actix_web::test]
    async fn not_found_without_the_config_section() {
        let response = call(None, Some(true)).await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
