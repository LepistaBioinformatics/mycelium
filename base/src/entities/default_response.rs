use crate::dtos::PaginatedRecord;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum DeletionResponseKind<T> {
    Deleted,
    NotDeleted(T, String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DeletionManyResponseKind<T> {
    Deleted(i64),
    NotDeleted(T, String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FetchResponseKind<T, U> {
    Found(T),
    NotFound(Option<U>),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FetchManyResponseKind<T> {
    Found(Vec<T>),
    FoundPaginated(PaginatedRecord<T>),
    NotFound,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum GetOrCreateResponseKind<T> {
    Created(T),
    NotCreated(T, String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CreateResponseKind<T> {
    Created(T),
    NotCreated(T, String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CreateManyResponseKind<T> {
    Created(Vec<T>),
    NotCreated(Vec<T>, String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UpdatingResponseKind<T> {
    Updated(T),
    NotUpdated(T, String),
}
