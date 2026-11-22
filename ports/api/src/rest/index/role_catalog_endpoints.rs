use crate::{
    federation::PreparedFederation, rest::federation::SignedPeerRequest,
};

use actix_web::{web, FromRequest, HttpRequest, HttpResponse};
use myc_core::{
    domain::{
        dtos::federation::RoleCatalogMode,
        entities::{GuestRoleFetching, InstanceSettingsFetching},
    },
    use_cases::federation::{
        is_federation_enabled, list_role_catalog, read_role_catalog_mode,
    },
};
use myc_http_tools::settings::ROLE_CATALOG_PATH;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.route(ROLE_CATALOG_PATH, web::get().to(get_role_catalog));
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Every guest role of this instance, for its peers (D-10, FED-55)
///
/// `federation.roleCatalog` decides who reads it: a request signed by a
/// declared peer (`peers`, the default), anyone (`public`), or nobody
/// (`off`, 404). Carries no tenant or account id. `404` too while
/// federation is absent or switched off.
///
#[tracing::instrument(name = "get_role_catalog", skip_all)]
pub async fn get_role_catalog(
    req: HttpRequest,
    payload: web::Payload,
    prepared: web::Data<Option<PreparedFederation>>,
    settings: web::Data<dyn InstanceSettingsFetching>,
    roles: web::Data<dyn GuestRoleFetching>,
) -> HttpResponse {
    if prepared.get_ref().is_none() {
        return HttpResponse::NotFound().finish();
    }
    if let Err(refused) = admit(&req, payload, settings.get_ref()).await {
        return refused;
    }

    match list_role_catalog(Box::new(roles.get_ref())).await {
        Ok(catalog) => HttpResponse::Ok().json(catalog),
        Err(err) => {
            tracing::error!(%err, "role catalog unreadable");
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Whether this caller may read the catalog, by the current mode
async fn admit(
    req: &HttpRequest,
    payload: web::Payload,
    settings: &dyn InstanceSettingsFetching,
) -> Result<(), HttpResponse> {
    let enabled = is_federation_enabled(Box::new(settings)).await;
    let mode = read_role_catalog_mode(Box::new(settings)).await;

    match (enabled, mode) {
        (Ok(true), Ok(RoleCatalogMode::Public)) => Ok(()),
        (Ok(true), Ok(RoleCatalogMode::Peers)) => {
            SignedPeerRequest::from_request(req, &mut payload.into_inner())
                .await
                .map(|_| ())
                .map_err(|err| err.as_response_error().error_response())
        }
        (Err(_), _) | (_, Err(_)) => {
            Err(HttpResponse::InternalServerError().finish())
        }
        _ => Err(HttpResponse::NotFound().finish()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::federation::peer_request_test_doubles::{
        federation_world, FederationWorld,
    };

    use actix_web::{http::StatusCode, test, App};
    use async_trait::async_trait;
    use myc_core::domain::dtos::{
        federation::PeerStatus,
        guest_role::{GuestRole, Permission},
    };
    use myc_http_tools::settings::PEER_REQUEST_SIGNATURE_KEY;
    use mycelium_base::{
        dtos::Children,
        entities::{FetchManyResponseKind, FetchResponseKind},
        utils::errors::MappedErrors,
    };
    use serde_json::Value;
    use std::sync::Arc;
    use uuid::Uuid;

    struct Roles;

    #[async_trait]
    impl GuestRoleFetching for Roles {
        async fn get(
            &self,
            _: Uuid,
        ) -> Result<FetchResponseKind<GuestRole, Uuid>, MappedErrors> {
            unreachable!()
        }

        async fn get_parent_by_child_id(
            &self,
            _: Uuid,
        ) -> Result<FetchResponseKind<GuestRole, Uuid>, MappedErrors> {
            unreachable!()
        }

        async fn list(
            &self,
            _: Option<String>,
            _: Option<String>,
            _: Option<bool>,
            _: Option<i32>,
            _: Option<i32>,
        ) -> Result<FetchManyResponseKind<GuestRole>, MappedErrors> {
            let child = GuestRole::new(
                Some(Uuid::from_u128(2)),
                "Reader".to_string(),
                None,
                Permission::Read,
                None,
                false,
            );
            let parent = GuestRole::new(
                Some(Uuid::from_u128(1)),
                "Editor".to_string(),
                Some("Edits documents".to_string()),
                Permission::Write,
                Some(Children::Ids(vec![Uuid::from_u128(2)])),
                true,
            );

            Ok(FetchManyResponseKind::Found(vec![parent, child]))
        }
    }

    async fn call(
        world: &FederationWorld,
        signature: Option<&str>,
    ) -> (StatusCode, Value) {
        let roles: Arc<dyn GuestRoleFetching> = Arc::new(Roles);
        let app = test::init_service(
            App::new()
                .configure(|config| world.configure(config))
                .app_data(web::Data::from(roles))
                .configure(configure),
        )
        .await;
        let mut request = test::TestRequest::get().uri(ROLE_CATALOG_PATH);
        if let Some(signature) = signature {
            request =
                request.insert_header((PEER_REQUEST_SIGNATURE_KEY, signature));
        }
        let response = test::call_service(&app, request.to_request()).await;
        let status = response.status();
        let body = test::read_body(response).await;

        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }

    fn world(mode: Option<&'static str>) -> FederationWorld {
        FederationWorld {
            role_catalog: mode,
            ..federation_world(PeerStatus::Active)
        }
    }

    fn signed_get(world: &FederationWorld) -> String {
        let issuer = world.peers.peer.instance_id;

        world.sign_request(issuer, "GET", ROLE_CATALOG_PATH, "jti-catalog", b"")
    }

    #[actix_web::test]
    async fn peers_mode_serves_a_declared_peer_only() {
        let world = world(None);

        assert_eq!(call(&world, None).await.0, StatusCode::UNAUTHORIZED);

        let (status, catalog) = call(&world, Some(&signed_get(&world))).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(catalog[0]["slug"], "editor");
        assert_eq!(catalog[0]["children"], serde_json::json!(["reader"]));
        assert_eq!(catalog[0]["permission"], "write");
        assert_eq!(catalog[0]["system"], true);
    }

    #[actix_web::test]
    async fn public_mode_serves_anyone_without_ids() {
        let world = world(Some("public"));

        let (status, catalog) = call(&world, None).await;
        let rendered = catalog.to_string();

        assert_eq!(status, StatusCode::OK);
        assert_eq!(catalog.as_array().unwrap().len(), 2);
        assert!(!rendered.contains(&Uuid::from_u128(1).to_string()));
        assert!(!rendered.contains("tenant"));
        assert!(!rendered.contains("account"));
    }

    #[actix_web::test]
    async fn off_mode_serves_nobody() {
        let world = world(Some("off"));

        assert_eq!(call(&world, None).await.0, StatusCode::NOT_FOUND);
        assert_eq!(
            call(&world, Some(&signed_get(&world))).await.0,
            StatusCode::NOT_FOUND
        );
    }
}
