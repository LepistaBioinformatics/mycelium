use super::{account::Account, email::Email, guest_role::GuestRole};

use chrono::{DateTime, Local};
use mycelium_base::{
    dtos::{Children, Parent},
    utils::errors::{dto_err, MappedErrors},
};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub struct GuestUser {
    /// The guest user id
    pub id: Option<Uuid>,

    /// The guest user email
    ///
    /// The email is used to identify the guest user connection with the target
    /// account.
    ///
    pub email: Email,

    /// The guest user role
    pub guest_role: Parent<GuestRole, Uuid>,

    /// The guesting date
    pub created: DateTime<Local>,

    /// The last updated date
    pub updated: Option<DateTime<Local>>,

    /// The account which the guest user is connected to
    pub accounts: Option<Children<Account, Uuid>>,

    /// The guest user is verified
    ///
    /// WHile the user is not verified, the user will not be able to access
    /// the account.
    ///
    pub was_verified: bool,
}

impl GuestUser {
    pub fn guest_role_id(&self) -> Result<Uuid, MappedErrors> {
        match self.guest_role.to_owned() {
            Parent::Id(id) => Ok(id),
            Parent::Record(record) => match record.id {
                Some(id) => Ok(id),
                None => dto_err("Guest role id is required").as_error(),
            },
        }
    }

    pub fn build_role_url(&self, base_url: String) -> Result<String, ()> {
        match self.guest_role.to_owned() {
            Parent::Id(id) => Ok(format!("{}/{}", base_url, id)),
            Parent::Record(record) => match record.id {
                Some(id) => Ok(format!("{}/{}", base_url, id)),
                None => Err(()),
            },
        }
    }

    pub fn new_unverified(
        email: Email,
        guest_role: Parent<GuestRole, Uuid>,
        accounts: Option<Children<Account, Uuid>>,
    ) -> Self {
        Self {
            id: None,
            email,
            guest_role,
            created: Local::now(),
            updated: None,
            accounts,
            was_verified: false,
        }
    }

    pub fn new_existing(
        id: Uuid,
        email: Email,
        guest_role: Parent<GuestRole, Uuid>,
        created: DateTime<Local>,
        updated: Option<DateTime<Local>>,
        accounts: Option<Children<Account, Uuid>>,
        was_verified: bool,
    ) -> Self {
        Self {
            id: Some(id),
            email,
            guest_role,
            created,
            updated,
            accounts,
            was_verified,
        }
    }
}
