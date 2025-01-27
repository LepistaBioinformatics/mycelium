use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, ToSchema, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TenantMetaKey {
    /// Federal Revenue Register
    ///
    /// The Federal Revenue Register is the register of the federal revenue
    /// of the tenant.
    FederalRevenueRegister,

    /// The type for the Federal Revenue Register
    ///
    /// In Brazil, the FRR is CNPJ. In the US, the FRR is EIN.
    FederalRevenueRegisterType,

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
    Custom(String),
}

impl Display for TenantMetaKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantMetaKey::FederalRevenueRegister => {
                write!(f, "federal_revenue_register")
            }
            TenantMetaKey::FederalRevenueRegisterType => {
                write!(f, "federal_revenue_register_type")
            }
            TenantMetaKey::Country => write!(f, "country"),
            TenantMetaKey::State => write!(f, "state"),
            TenantMetaKey::Province => write!(f, "province"),
            TenantMetaKey::City => write!(f, "city"),
            TenantMetaKey::Address1 => write!(f, "address1"),
            TenantMetaKey::Address2 => write!(f, "address2"),
            TenantMetaKey::ZipCode => write!(f, "zip_code"),
            TenantMetaKey::Custom(key) => write!(f, "custom:{}", key),
        }
    }
}

impl FromStr for TenantMetaKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("custom:") {
            return Ok(TenantMetaKey::Custom(s[7..].to_owned()));
        }

        match s {
            "federal_revenue_register" => {
                Ok(TenantMetaKey::FederalRevenueRegister)
            }
            "federal_revenue_register_type" => {
                Ok(TenantMetaKey::FederalRevenueRegisterType)
            }
            "country" => Ok(TenantMetaKey::Country),
            "state" => Ok(TenantMetaKey::State),
            "province" => Ok(TenantMetaKey::Province),
            "city" => Ok(TenantMetaKey::City),
            "address1" => Ok(TenantMetaKey::Address1),
            "address2" => Ok(TenantMetaKey::Address2),
            "zip_code" => Ok(TenantMetaKey::ZipCode),
            _ => Err(format!("Invalid key: {}", s)),
        }
    }
}

impl Serialize for TenantMetaKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            TenantMetaKey::Custom(key) => serializer
                .serialize_str(format!("custom:{key}", key = key).as_str()),
            _ => serializer.serialize_str(&self.to_string()),
        }
    }
}
