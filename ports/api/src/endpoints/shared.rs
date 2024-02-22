use crate::settings::MYCELIUM_API_SCOPE;

use myc_core::domain::actors::DefaultActor;
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
    Staffs,
}

impl Display for UrlScope {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            UrlScope::Health => write!(f, "health"),
            UrlScope::Standards => write!(f, "std"),
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

impl UrlGroup {
    pub fn with_scope(&self, scope: UrlScope) -> String {
        format!("{}/{}", scope.build_myc_path(), self.to_owned())
    }

    pub fn with_scoped_actor(
        &self,
        scope: UrlScope,
        actor: DefaultActor,
    ) -> String {
        format!("{}/{}/{}", scope.build_myc_path(), actor, self.to_owned())
    }
}
