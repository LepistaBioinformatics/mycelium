mod meta;
mod status;

use chrono::{DateTime, Local};
pub use meta::TenantMetaKey;
pub use status::TenantStatus;

use super::{account::Account, profile::Owner, tag::Tag};
use mycelium_base::{
    dtos::{Children, Parent},
    utils::errors::{dto_err, MappedErrors},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

pub type TenantMeta = HashMap<TenantMetaKey, String>;

pub type TenantId = Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tenant {
    pub id: Option<TenantId>,
    pub name: String,
    pub description: Option<String>,

    /// The owner of the tenant
    ///
    /// This is the email of the tenant owner, which is also the pub owner. The
    /// tenant owner should be set on tenant creation.
    pub owners: Children<Owner, Uuid>,

    /// The tenant manager
    ///
    /// The account of the tenant manager.
    pub manager: Option<Parent<Account, Uuid>>,

    /// The tags of the tenant
    ///
    /// This is the list of tags of the tenant. The tags are used to categorize
    /// the tenant. The tags are used to categorize the tenant.
    pub tags: Option<Vec<Tag>>,

    /// Meta information
    ///
    /// This is the meta information of the tenant. The meta information is a
    /// key-value pair of string. The key is the name of the meta information,
    /// and the value is the value of the meta information.
    pub meta: Option<TenantMeta>,

    /// The status of the tenant
    ///
    /// This is the status of the tenant. The status is a key-value pair of
    /// string. The key is the name of the status (defined in `StatusKey`), and
    /// the value is the value of the status.
    pub status: Option<Vec<TenantStatus>>,

    /// The date and time the tenant was created
    pub created: DateTime<Local>,

    /// The date and time the tenant was last updated
    pub updated: Option<DateTime<Local>>,
}

impl Tenant {
    pub fn new_with_owners(
        name: String,
        description: Option<String>,
        owners: Children<Owner, Uuid>,
    ) -> Self {
        Self {
            id: None,
            name,
            description,
            owners,
            manager: None,
            tags: None,
            meta: None,
            status: None,
            created: Local::now(),
            updated: None,
        }
    }

    pub fn tenant_string_or_error(&self) -> Result<String, MappedErrors> {
        if let Some(id) = self.id {
            Ok(format!("tenant/{}", id.to_string()))
        } else {
            dto_err("Unable to format owner name").as_error()
        }
    }
}
