use actix_web::{
    http::header::{CACHE_CONTROL, CONTENT_TYPE},
    web, HttpResponse,
};
use myc_http_tools::{
    models::PublishedProfileKeys, settings::PROFILE_JWKS_PATH,
};

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.route(PROFILE_JWKS_PATH, web::get().to(get_profile_signing_jwks));
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

/// Public keys downstream services use to verify signed profiles
///
/// Serves the JWKS prepared at boot from `[core.profileSigning]`: one
/// Ed25519 JWK per configured key version, never private material. Public
/// and unauthenticated by design: the keys only verify. `404` when profile
/// signing is not configured.
///
#[tracing::instrument(name = "get_profile_signing_jwks", skip_all)]
pub async fn get_profile_signing_jwks(
    published: Option<web::Data<PublishedProfileKeys>>,
) -> HttpResponse {
    let Some(PublishedProfileKeys::Enabled { body, max_age }) =
        published.as_ref().map(|data| data.get_ref())
    else {
        return HttpResponse::NotFound().finish();
    };

    HttpResponse::Ok()
        .insert_header((CONTENT_TYPE, "application/jwk-set+json"))
        .insert_header((
            CACHE_CONTROL,
            PublishedProfileKeys::cache_control(*max_age),
        ))
        .body(body.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{dev::ServiceResponse, http::StatusCode, test, App};
    use myc_core::domain::dtos::profile_signature::{
        JwksMaxAgeSeconds, SigningKeyVersion,
    };
    use myc_http_tools::models::Ed25519PublicJwk;
    use serde_json::Value;

    fn enabled() -> PublishedProfileKeys {
        let version = SigningKeyVersion::try_from(1).unwrap();

        PublishedProfileKeys::new(
            &[Ed25519PublicJwk::new(version, [7; 32])],
            JwksMaxAgeSeconds::try_from(600).unwrap(),
        )
    }

    async fn call(published: PublishedProfileKeys) -> ServiceResponse {
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(published))
                .configure(configure),
        )
        .await;
        let request = test::TestRequest::get().uri(PROFILE_JWKS_PATH);

        test::call_service(&app, request.to_request()).await
    }

    #[actix_web::test]
    async fn serves_the_prepared_key_set_with_its_cache_lifetime() {
        let response = call(enabled()).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(CACHE_CONTROL).unwrap(),
            "public, max-age=600"
        );
        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "application/jwk-set+json"
        );

        let document: Value = test::read_body_json(response).await;
        assert_eq!(document["keys"][0]["kid"], "1");
        assert!(document["keys"][0].get("d").is_none());
    }

    #[actix_web::test]
    async fn not_found_when_signing_is_disabled() {
        let response = call(PublishedProfileKeys::Disabled).await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
