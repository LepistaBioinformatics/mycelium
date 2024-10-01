use super::{account::Account, tag::Tag, user::User};

use mycelium_base::dtos::{Children, GenericMapValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, Eq, Hash, PartialEq,
)]
#[serde(rename_all = "camelCase")]
pub enum MetaKey {
    /// Federal Revenue Register
    ///
    /// The Federal Revenue Register is the register of the federal revenue
    /// of the tenant.
    FederalRevenueRegister,

    /// The type for the Federal Revenue Register
    ///
    /// In Brazil, the FRR is CNPJ. In the US, the FRR is EIN.
    FRRType,

    /// The Country
    ///
    /// The country of the tenant.
    Country,

    /// To specify any other meta key
    Other(String),
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,

    /// The owner of the tenant
    ///
    /// This is the email of the tenant owner, which is also the pub owner. The
    /// tenant owner should be set on tenant creation.
    pub owners: Children<User, Uuid>,

    /// The tenant admins
    ///
    /// This is the list of tenant admins. The tenant admins are the users who
    /// have the tenant manager role.
    pub managers: Children<Account, Uuid>,

    /// Meta information
    ///
    /// This is the meta information of the tenant. The meta information is a
    /// key-value pair of string. The key is the name of the meta information,
    /// and the value is the value of the meta information.
    pub meta: Option<HashMap<MetaKey, GenericMapValue<String>>>,

    /// The tags of the tenant
    ///
    /// This is the list of tags of the tenant. The tags are used to categorize
    /// the tenant. The tags are used to categorize the tenant.
    pub tags: Option<Vec<Tag>>,
}
