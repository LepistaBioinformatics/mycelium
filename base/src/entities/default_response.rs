use crate::dtos::PaginatedRecord;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum DeletionResponseKind<T>
where
    T: Serialize,
{
    Deleted,
    NotDeleted(T, String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DeletionManyResponseKind<T>
where
    T: Serialize,
{
    Deleted(i64),
    NotDeleted(T, String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FetchResponseKind<T, U>
where
    T: Serialize,
{
    Found(T),
    NotFound(Option<U>),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FetchManyResponseKind<T>
where
    T: Serialize,
{
    Found(Vec<T>),
    FoundPaginated(PaginatedRecord<T>),
    NotFound,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum GetOrCreateResponseKind<T>
where
    T: Serialize,
{
    Created(T),
    NotCreated(T, String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CreateResponseKind<T>
where
    T: Serialize,
{
    Created(T),
    NotCreated(T, String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CreateManyResponseKind<T>
where
    T: Serialize,
{
    Created(Vec<T>),
    NotCreated(Vec<T>, String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UpdatingResponseKind<T>
where
    T: Serialize,
{
    Updated(T),
    NotUpdated(T, String),
}
