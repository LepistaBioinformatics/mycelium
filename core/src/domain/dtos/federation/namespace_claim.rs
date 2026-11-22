use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

/// What kind of foreign id a peer claims (I-1)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ForeignIdKind {
    Tenant,
    Account,
}

impl ForeignIdKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tenant => "tenant",
            Self::Account => "account",
        }
    }
}

impl fmt::Display for ForeignIdKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ForeignIdKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "tenant" => Ok(Self::Tenant),
            "account" => Ok(Self::Account),
            other => Err(format!("unknown foreign id kind: {other}")),
        }
    }
}

/// A foreign id and the peer that first asserted it (FED-54)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NamespaceClaim {
    pub foreign_id: Uuid,
    pub peer_id: Uuid,
    pub kind: ForeignIdKind,
}
