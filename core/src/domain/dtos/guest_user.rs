use super::{account::Account, email::Email, guest_role::GuestRole};

use chrono::{DateTime, Local};
use mycelium_base::dtos::{Children, Parent};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GuestUser {
    pub id: Option<Uuid>,

    pub email: Email,
    pub guest_role: Parent<GuestRole, Uuid>,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
    pub accounts: Option<Children<Account, Uuid>>,
}

impl GuestUser {
    pub fn build_role_url(&self, base_url: String) -> Result<String, ()> {
        match self.guest_role.to_owned() {
            Parent::Id(id) => Ok(format!("{}/{}", base_url, id)),
            Parent::Record(record) => match record.id {
                Some(id) => Ok(format!("{}/{}", base_url, id.to_string())),
                None => Err(()),
            },
        }
    }
}
