use crate::settings::MYCELIUM_API_SCOPE;

use serde::Deserialize;
use std::fmt::{Display, Formatter, Result as FmtResult};
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

pub(crate) enum UrlScopes {
    Health,
    Standards,
    Staffs,
}

impl Display for UrlScopes {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            UrlScopes::Health => write!(f, "health"),
            UrlScopes::Standards => write!(f, "std"),
            UrlScopes::Staffs => write!(f, "staffs"),
        }
    }
}

pub fn build_scoped_path(scope: UrlScopes) -> String {
    format!("/{}/{}", MYCELIUM_API_SCOPE, scope)
}

pub(crate) enum UrlGroup {
    Accounts,
    GuestRoles,
    Guests,
    Roles,
    Users,
    Webhooks,
    ErrorCodes,
    Profile,
}

impl Display for UrlGroup {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            UrlGroup::Accounts => write!(f, "accounts"),
            UrlGroup::GuestRoles => write!(f, "guest-roles"),
            UrlGroup::Guests => write!(f, "guests"),
            UrlGroup::Roles => write!(f, "roles"),
            UrlGroup::Users => write!(f, "users"),
            UrlGroup::Webhooks => write!(f, "webhooks"),
            UrlGroup::ErrorCodes => write!(f, "error-codes"),
            UrlGroup::Profile => write!(f, "profile"),
        }
    }
}

pub fn build_scoped_group(scope: UrlScopes, group: UrlGroup) -> String {
    format!("{}/{}", build_scoped_path(scope), group)
}
