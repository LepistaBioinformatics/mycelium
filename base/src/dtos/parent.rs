use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};
use utoipa::ToSchema;

/// A parent record
///
/// This enumerator allow represents the parent elements using their primary
/// key (Id option) or the true record (Record option).
#[derive(
    Clone, Debug, Deserialize, Serialize, Eq, Hash, PartialEq, ToSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum Parent<T, U> {
    Record(T),
    Id(U),
}

impl<T, U> Display for Parent<T, U>
where
    T: Display,
    U: Display,
{
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Parent::Record(_) => write!(f, "record"),
            Parent::Id(_) => write!(f, "id"),
        }
    }
}

#[derive(
    Clone, Debug, Deserialize, Serialize, Eq, Hash, PartialEq, ToSchema,
)]
#[serde(untagged, rename_all = "camelCase")]
pub enum UntaggedParent<T, U> {
    Record(T),
    Id(U),
}

impl<T, U> Display for UntaggedParent<T, U>
where
    T: Display,
    U: Display,
{
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            UntaggedParent::Record(_) => write!(f, "record"),
            UntaggedParent::Id(_) => write!(f, "id"),
        }
    }
}
