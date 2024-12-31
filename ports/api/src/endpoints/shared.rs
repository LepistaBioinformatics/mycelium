use crate::settings::{ADMIN_API_SCOPE, ROLE_SCOPED_API_SCOPE};

use actix_web::dev::ServiceRequest;
use myc_http_tools::{settings::DEFAULT_MYCELIUM_ROLE_KEY, SystemActor};
use oauth2::http::HeaderName;
use serde::Deserialize;
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};
use tracing::error;
use utoipa::IntoParams;

/// Insert the role header into the request
///
/// This function is useful to insert the role header into the request before
/// sending it to the downstream services. It is used to propagate the role
/// of the actor to the downstream services as a middleware.
///
pub(crate) fn insert_role_header(
    mut req: ServiceRequest,
    actors: Vec<SystemActor>,
) -> ServiceRequest {
    let header_name = match HeaderName::from_str(DEFAULT_MYCELIUM_ROLE_KEY) {
        Ok(header_name) => header_name,
        Err(err) => {
            error!("Failed to parse header name: {err}");

            return req;
        }
    };

    let header_value = match (match serde_json::to_string(
        &actors
            .iter()
            .map(|actor| actor.to_string())
            .collect::<Vec<String>>(),
    ) {
        Ok(header_value_) => header_value_,
        Err(err) => {
            error!("Failed to serialize header value: {err}");

            return req;
        }
    })
    .parse()
    {
        Ok(header_value) => header_value,
        Err(err) => {
            error!("Failed to parse header value: {err}");

            return req;
        }
    };

    req.headers_mut().insert(header_name, header_value);

    req
}

/// Build the actor context
///
/// This function is useful to build the actor in OpenAPI documentation.
///
pub(crate) fn build_actor_context(
    actor: SystemActor,
    group: UrlGroup,
) -> String {
    group.with_scoped_actor(UrlScope::RoleScoped, actor)
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct PaginationParams {
    pub skip: Option<i32>,
    pub page_size: Option<i32>,
}

pub enum UrlScope {
    Health,
    RoleScoped,
    Managers,
    Staffs,
    Service,
}

impl Display for UrlScope {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            UrlScope::Health => write!(f, "health"),
            UrlScope::RoleScoped => write!(f, "{}", ROLE_SCOPED_API_SCOPE),
            UrlScope::Managers => write!(f, "managers"),
            UrlScope::Staffs => write!(f, "staffs"),
            UrlScope::Service => write!(f, "svc"),
        }
    }
}

impl UrlScope {
    pub fn build_myc_path(&self) -> String {
        format!("/{}/{}", ADMIN_API_SCOPE, self.to_owned())
    }
}

pub enum UrlGroup {
    Accounts,
    ErrorCodes,
    GuestRoles,
    Guests,
    Meta,
    Owners,
    Profile,
    Roles,
    Routes,
    Services,
    Tags,
    Tenants,
    Tokens,
    Users,
    Webhooks,
}

impl Display for UrlGroup {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            UrlGroup::Accounts => write!(f, "accounts"),
            UrlGroup::ErrorCodes => write!(f, "error-codes"),
            UrlGroup::GuestRoles => write!(f, "guest-roles"),
            UrlGroup::Guests => write!(f, "guests"),
            UrlGroup::Meta => write!(f, "meta"),
            UrlGroup::Owners => write!(f, "owners"),
            UrlGroup::Profile => write!(f, "profile"),
            UrlGroup::Roles => write!(f, "roles"),
            UrlGroup::Routes => write!(f, "routes"),
            UrlGroup::Services => write!(f, "services"),
            UrlGroup::Tags => write!(f, "tags"),
            UrlGroup::Tenants => write!(f, "tenants"),
            UrlGroup::Tokens => write!(f, "tokens"),
            UrlGroup::Users => write!(f, "users"),
            UrlGroup::Webhooks => write!(f, "webhooks"),
        }
    }
}

impl UrlGroup {
    pub fn with_scoped_actor(
        &self,
        scope: UrlScope,
        actor: SystemActor,
    ) -> String {
        format!("{}/{}/{}", scope.build_myc_path(), actor, self.to_owned())
    }
}
