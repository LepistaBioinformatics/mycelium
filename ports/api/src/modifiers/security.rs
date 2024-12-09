use myc_http_tools::settings::DEFAULT_CONNECTION_STRING_KEY;
use serde::Serialize;
use utoipa::{
    openapi::{
        self,
        security::{
            ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme,
        },
    },
    Modify,
};

#[derive(Debug, Serialize)]
pub(crate) struct MyceliumSecurity;

impl Modify for MyceliumSecurity {
    fn modify(&self, openapi: &mut openapi::OpenApi) {
        if let Some(schema) = openapi.components.as_mut() {
            schema.add_security_scheme(
                "Bearer",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("Bearer")
                        .build(),
                ),
            );

            schema.add_security_scheme(
                "ConnectionString",
                SecurityScheme::ApiKey(ApiKey::Header(
                    ApiKeyValue::with_description(
                        DEFAULT_CONNECTION_STRING_KEY,
                        "A valid Mycelium Connection String",
                    ),
                )),
            );
        }
    }
}
