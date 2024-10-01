use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};
use utoipa::ToSchema;

/// A children record
///
/// This enumerator allow represents the children elements using their primary
/// keys (Ids option) or the true records (Record option).
#[derive(
    Clone, Debug, Deserialize, Serialize, Eq, Hash, PartialEq, ToSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum Children<T, U> {
    Records(Vec<T>),
    Ids(Vec<U>),
}

impl<T, U> Display for Children<T, U>
where
    T: Display,
    U: Display,
{
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Children::Records(_) => write!(f, "records"),
            Children::Ids(_) => write!(f, "ids"),
        }
    }
}

#[derive(
    Clone, Debug, Deserialize, Serialize, Eq, Hash, PartialEq, ToSchema,
)]
#[serde(untagged, rename_all = "camelCase")]
pub enum UntaggedChildren<T, U> {
    Records(Vec<T>),
    Ids(Vec<U>),
}

impl<T, U> Display for UntaggedChildren<T, U>
where
    T: Display,
    U: Display,
{
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            UntaggedChildren::Records(_) => write!(f, "records"),
            UntaggedChildren::Ids(_) => write!(f, "ids"),
        }
    }
}
