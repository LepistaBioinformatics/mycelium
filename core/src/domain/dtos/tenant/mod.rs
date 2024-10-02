mod meta;
mod status;

pub use meta::TenantMeta;
pub use status::TenantStatus;

use super::{account::Account, profile::Owner, tag::Tag};
use mycelium_base::dtos::Children;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tenant {
    pub id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,

    /// The owner of the tenant
    ///
    /// This is the email of the tenant owner, which is also the pub owner. The
    /// tenant owner should be set on tenant creation.
    pub owners: Children<Owner, Uuid>,

    /// The tenant admins
    ///
    /// This is the list of tenant admins. The tenant admins are the users who
    /// have the tenant manager role.
    pub managers: Option<Children<Account, Uuid>>,

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
    pub meta: Option<HashSet<TenantMeta>>,

    /// The status of the tenant
    ///
    /// This is the status of the tenant. The status is a key-value pair of
    /// string. The key is the name of the status (defined in `StatusKey`), and
    /// the value is the value of the status.
    pub status: Option<HashSet<TenantStatus>>,
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
            managers: None,
            tags: None,
            meta: None,
            status: None,
        }
    }
}
