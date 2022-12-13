use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ParentEnum<T, U> {
    Id(T),
    Record(U),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChildrenEnum<T, U> {
    Ids(Vec<T>),
    Records(Vec<U>),
}
