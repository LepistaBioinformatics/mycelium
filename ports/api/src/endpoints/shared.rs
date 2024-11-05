use crate::settings::MYCELIUM_API_SCOPE;

use myc_http_tools::ActorName;
use serde::Deserialize;
use std::fmt::{Display, Formatter, Result as FmtResult};
use utoipa::IntoParams;

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct PaginationParams {
    pub skip: Option<i32>,
    pub page_size: Option<i32>,
}

pub enum UrlScope {
    Health,
    Standards,
    Managers,
    Staffs,
}

impl Display for UrlScope {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            UrlScope::Health => write!(f, "health"),
            UrlScope::Standards => write!(f, "std"),
            UrlScope::Managers => write!(f, "managers"),
            UrlScope::Staffs => write!(f, "staffs"),
        }
    }
}

impl UrlScope {
    pub fn build_myc_path(&self) -> String {
        format!("/{}/{}", MYCELIUM_API_SCOPE, self.to_owned())
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
            UrlGroup::Tags => write!(f, "tags"),
            UrlGroup::Tenants => write!(f, "tenants"),
            UrlGroup::Tokens => write!(f, "tokens"),
            UrlGroup::Users => write!(f, "users"),
            UrlGroup::Webhooks => write!(f, "webhooks"),
        }
    }
}

impl UrlGroup {
    pub fn with_scope(&self, scope: UrlScope) -> String {
        format!("{}/{}", scope.build_myc_path(), self.to_owned())
    }

    pub fn with_scoped_actor(
        &self,
        scope: UrlScope,
        actor: ActorName,
    ) -> String {
        format!("{}/{}/{}", scope.build_myc_path(), actor, self.to_owned())
    }
}
