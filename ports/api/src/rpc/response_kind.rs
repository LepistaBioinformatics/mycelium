//! Converte os tipos default_response (FetchResponseKind, GetOrCreateResponseKind, etc.)
//! para o formato de resultado RPC, espelhando o comportamento da API REST:
//! Found/Created/Updated → result = corpo; NotFound/Deleted → result = null;
//! NotCreated/NotUpdated/NotDeleted → erro JSON-RPC.

use super::types::{self, JsonRpcError};
use mycelium_base::{
    dtos::PaginatedRecord,
    entities::{
        CreateResponseKind, DeletionResponseKind, FetchManyResponseKind,
        FetchResponseKind, GetOrCreateResponseKind, UpdatingResponseKind,
    },
};
use serde::Serialize;

/// Converte `FetchResponseKind` em resultado RPC: Found(t) → Ok(json(t)), NotFound → Ok(null).
pub fn fetch_response_kind_to_result<T: Serialize, U>(
    response: FetchResponseKind<T, U>,
) -> Result<serde_json::Value, JsonRpcError> {
    match response {
        FetchResponseKind::Found(res) => {
            serde_json::to_value(res).map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        FetchResponseKind::NotFound(_) => Ok(serde_json::Value::Null),
    }
}

/// Converte `FetchManyResponseKind` em resultado RPC: Found/FoundPaginated → Ok(json), NotFound → Ok(null).
pub fn fetch_many_response_kind_to_result<T: Serialize>(
    response: FetchManyResponseKind<T>,
) -> Result<serde_json::Value, JsonRpcError> {
    match response {
        FetchManyResponseKind::Found(res) => {
            serde_json::to_value(res).map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        FetchManyResponseKind::FoundPaginated {
            count,
            skip,
            size,
            records,
        } => serde_json::to_value(PaginatedRecord {
            count,
            skip,
            size,
            records,
        })
        .map_err(|e| JsonRpcError {
            code: types::codes::INTERNAL_ERROR,
            message: e.to_string(),
            data: None,
        }),
        FetchManyResponseKind::NotFound => Ok(serde_json::Value::Null),
    }
}

/// Converte `CreateResponseKind` em resultado RPC: Created(t) → Ok(json(t)), NotCreated → Err.
pub fn create_response_kind_to_result<T: Serialize>(
    response: CreateResponseKind<T>,
) -> Result<serde_json::Value, JsonRpcError> {
    match response {
        CreateResponseKind::Created(res) => {
            serde_json::to_value(res).map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        CreateResponseKind::NotCreated(_, msg) => Err(JsonRpcError {
            code: types::codes::INVALID_PARAMS,
            message: msg,
            data: None,
        }),
    }
}

/// Converte `GetOrCreateResponseKind` em resultado RPC: Created(t) → Ok(json(t)), NotCreated → Err.
pub fn get_or_create_response_kind_to_result<T: Serialize>(
    response: GetOrCreateResponseKind<T>,
) -> Result<serde_json::Value, JsonRpcError> {
    match response {
        GetOrCreateResponseKind::Created(res) => serde_json::to_value(res)
            .map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            }),
        GetOrCreateResponseKind::NotCreated(_, msg) => Err(JsonRpcError {
            code: types::codes::INVALID_PARAMS,
            message: msg,
            data: None,
        }),
    }
}

/// Converte `UpdatingResponseKind` em resultado RPC: Updated(t) → Ok(json(t)), NotUpdated → Err.
pub fn updating_response_kind_to_result<T: Serialize>(
    response: UpdatingResponseKind<T>,
) -> Result<serde_json::Value, JsonRpcError> {
    match response {
        UpdatingResponseKind::Updated(res) => serde_json::to_value(res)
            .map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            }),
        UpdatingResponseKind::NotUpdated(_, msg) => Err(JsonRpcError {
            code: types::codes::INVALID_PARAMS,
            message: msg,
            data: None,
        }),
    }
}

/// Converte `DeletionResponseKind` em resultado RPC: Deleted → Ok(null), NotDeleted → Err.
pub fn delete_response_kind_to_result<T: Serialize>(
    response: DeletionResponseKind<T>,
) -> Result<serde_json::Value, JsonRpcError> {
    match response {
        DeletionResponseKind::Deleted => Ok(serde_json::Value::Null),
        DeletionResponseKind::NotDeleted(_, msg) => Err(JsonRpcError {
            code: types::codes::INVALID_PARAMS,
            message: msg,
            data: None,
        }),
    }
}
