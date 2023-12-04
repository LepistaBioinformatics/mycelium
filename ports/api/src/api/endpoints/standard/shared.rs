use crate::endpoints::shared::{build_scoped_path, UrlScopes};

use myc_core::domain::actors::DefaultActor;
use std::fmt::{Display, Formatter, Result as FmtResult};

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

pub fn build_actor_context(actor: DefaultActor, group: UrlGroup) -> String {
    format!(
        "{}/{}s/{}",
        build_scoped_path(UrlScopes::Standards),
        actor,
        group
    )
}
