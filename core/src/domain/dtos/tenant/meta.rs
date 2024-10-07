use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use utoipa::ToSchema;

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, Eq, Hash, PartialEq,
)]
#[serde(rename_all = "camelCase")]
pub enum TenantMetaKey {
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

    /// The State
    ///
    /// The state of the tenant.
    State,

    /// The Province
    ///
    /// The province of the tenant.
    Province,

    /// The City
    ///
    /// The city of the tenant.
    City,

    /// The Address 1
    ///
    /// The first address of the tenant.
    Address1,

    /// The Address 2
    ///
    /// The second address of the tenant.
    Address2,

    /// The Zip Code
    ///
    /// The zip code of the tenant.
    ZipCode,

    /// To specify any other meta key
    ///
    /// Specify any other meta key that is not listed here.
    #[serde(untagged)]
    Other(String),
}

impl Display for TenantMetaKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantMetaKey::FederalRevenueRegister => {
                write!(f, "FederalRevenueRegister")
            }
            TenantMetaKey::FRRType => write!(f, "FRRType"),
            TenantMetaKey::Country => write!(f, "Country"),
            TenantMetaKey::State => write!(f, "State"),
            TenantMetaKey::Province => write!(f, "Province"),
            TenantMetaKey::City => write!(f, "City"),
            TenantMetaKey::Address1 => write!(f, "Address1"),
            TenantMetaKey::Address2 => write!(f, "Address2"),
            TenantMetaKey::ZipCode => write!(f, "ZipCode"),
            TenantMetaKey::Other(key) => write!(f, "{}", key),
        }
    }
}

impl FromStr for TenantMetaKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "FederalRevenueRegister" => {
                Ok(TenantMetaKey::FederalRevenueRegister)
            }
            "FRRType" => Ok(TenantMetaKey::FRRType),
            "Country" => Ok(TenantMetaKey::Country),
            "State" => Ok(TenantMetaKey::State),
            "Province" => Ok(TenantMetaKey::Province),
            "City" => Ok(TenantMetaKey::City),
            "Address1" => Ok(TenantMetaKey::Address1),
            "Address2" => Ok(TenantMetaKey::Address2),
            "ZipCode" => Ok(TenantMetaKey::ZipCode),
            val => Ok(TenantMetaKey::Other(val.to_owned())),
        }
    }
}
