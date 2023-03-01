use serde::Deserialize;
use utoipa::{
    openapi::security::{
        AuthorizationCode, ClientCredentials, Flow, Implicit, OAuth2, Scopes,
        SecurityScheme,
    },
    IntoParams, Modify,
};

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct PaginationParams {
    pub skip: Option<i32>,
    pub page_size: Option<i32>,
}

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();

        let authorization_url =
            "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";

        let token_url =
            "https://login.microsoftonline.com/common/oauth2/v2.0/token";

        let scopes =
            [("https://graph.microsoft.com/openid", "Read OpenID profile")];

        components.add_security_scheme(
            "oauth2",
            SecurityScheme::OAuth2(OAuth2::with_description(
                [
                    Flow::Implicit(Implicit::new(
                        authorization_url,
                        //"http://localhost:8080/myc/auth",
                        Scopes::from_iter(scopes),
                    )),
                    Flow::ClientCredentials(ClientCredentials::new(
                        token_url,
                        Scopes::from_iter(scopes),
                    )),
                    Flow::AuthorizationCode(AuthorizationCode::new(
                        authorization_url,
                        token_url,
                        Scopes::from_iter(scopes),
                    )),
                ],
                "Default Users Oauth2 Flow",
            )),
        )
    }
}
