use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, Eq, Hash, PartialEq,
)]
#[serde(rename_all = "camelCase")]
pub enum TenantMeta {
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
    Other,
}
