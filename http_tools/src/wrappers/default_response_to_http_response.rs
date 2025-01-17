use crate::utils::HttpJsonResponse;

use actix_web::HttpResponse;
use myc_core::domain::dtos::native_error_codes::NativeErrorCodes::*;
use mycelium_base::{
    entities::{
        CreateManyResponseKind, CreateResponseKind, DeletionManyResponseKind,
        DeletionResponseKind, FetchManyResponseKind, FetchResponseKind,
        GetOrCreateResponseKind, UpdatingResponseKind,
    },
    utils::errors::MappedErrors,
};
use serde::Serialize;
use std::vec;
use tracing::error;

/// Wraps a `CreateResponseKind` into a `HttpResponse`
pub fn create_response_kind<T: Serialize>(
    response: CreateResponseKind<T>,
) -> HttpResponse {
    match response {
        CreateResponseKind::Created(res) => HttpResponse::Created().json(res),
        CreateResponseKind::NotCreated(res, msg) => {
            let json = HttpJsonResponse::new_message(msg);

            match json.with_serializable_body(res) {
                Ok(json) => HttpResponse::BadRequest().json(json),
                Err(err) => err,
            }
        }
    }
}

/// Wraps a `CreateManyResponseKind` into a `HttpResponse`
pub fn create_many_response_kind<T: Serialize>(
    response: CreateManyResponseKind<T>,
) -> HttpResponse {
    match response {
        CreateManyResponseKind::Created(res) => {
            match HttpJsonResponse::new_vec_body(res) {
                Ok(json) => HttpResponse::Created().json(json),
                Err(err) => err,
            }
        }
        CreateManyResponseKind::NotCreated(res, msg) => {
            let json = HttpJsonResponse::new_message(msg);

            match json.with_serializable_body(res) {
                Ok(json) => HttpResponse::BadRequest().json(json),
                Err(err) => err,
            }
        }
    }
}

/// Wraps a `GetOrCreateResponseKind` into a `HttpResponse`
pub fn get_or_create_response_kind<T: Serialize>(
    response: GetOrCreateResponseKind<T>,
) -> HttpResponse {
    match response {
        GetOrCreateResponseKind::Created(res) => {
            HttpResponse::Created().json(res)
        }
        GetOrCreateResponseKind::NotCreated(res, msg) => {
            let json = HttpJsonResponse::new_message(msg);

            match json.with_serializable_body(res) {
                Ok(json) => HttpResponse::BadRequest().json(json),
                Err(err) => err,
            }
        }
    }
}

/// Wraps a `DeletionResponseKind` into a `HttpResponse`
pub fn delete_response_kind<T: Serialize>(
    response: DeletionResponseKind<T>,
) -> HttpResponse {
    match response {
        DeletionResponseKind::Deleted => HttpResponse::NoContent().finish(),
        DeletionResponseKind::NotDeleted(res, msg) => {
            let json = HttpJsonResponse::new_message(msg);

            match json.with_serializable_body(res) {
                Ok(json) => HttpResponse::BadRequest().json(json),
                Err(err) => err,
            }
        }
    }
}

/// Wraps a `DeletionManyResponseKind` into a `HttpResponse`
pub fn delete_many_response_kind<T: Serialize>(
    response: DeletionManyResponseKind<T>,
) -> HttpResponse {
    match response {
        DeletionManyResponseKind::Deleted(res) => {
            match HttpJsonResponse::new_body(res) {
                Ok(json) => HttpResponse::Accepted().json(json),
                Err(err) => err,
            }
        }
        DeletionManyResponseKind::NotDeleted(res, msg) => {
            let json = HttpJsonResponse::new_message(msg);

            match json.with_serializable_body(res) {
                Ok(json) => HttpResponse::BadRequest().json(json),
                Err(err) => err,
            }
        }
    }
}

/// Wraps a `FetchResponseKind` into a `HttpResponse`
pub fn fetch_response_kind<T: Serialize, U: ToString + Serialize>(
    response: FetchResponseKind<T, U>,
) -> HttpResponse {
    match response {
        FetchResponseKind::Found(res) => HttpResponse::Ok().json(res),
        FetchResponseKind::NotFound(res) => {
            if let Some(res) = res {
                return HttpResponse::NotFound()
                    .json(HttpJsonResponse::new_message(res));
            }

            HttpResponse::NoContent().finish()
        }
    }
}

/// Wraps a `FetchManyResponseKind` into a `HttpResponse`
pub fn fetch_many_response_kind<T: Serialize>(
    response: FetchManyResponseKind<T>,
) -> HttpResponse {
    match response {
        FetchManyResponseKind::Found(res) => HttpResponse::Ok().json(res),
        FetchManyResponseKind::FoundPaginated(res) => {
            HttpResponse::Ok().json(res)
        }
        FetchManyResponseKind::NotFound => HttpResponse::NoContent().finish(),
    }
}

/// Wraps a `UpdatingResponseKind` into a `HttpResponse`
pub fn updating_response_kind<T: Serialize>(
    response: UpdatingResponseKind<T>,
) -> HttpResponse {
    match response {
        UpdatingResponseKind::Updated(res) => {
            HttpResponse::Accepted().json(res)
        }
        UpdatingResponseKind::NotUpdated(res, msg) => {
            let json = HttpJsonResponse::new_message(msg);

            match json.with_serializable_body(res) {
                Ok(json) => HttpResponse::BadRequest().json(json),
                Err(err) => err,
            }
        }
    }
}

/// Map a `MappedErrors` into a `HttpResponse`
///
/// This function maps the error codes to the corresponding `HttpResponse`
/// during the http request handling.
///
pub fn handle_mapped_error(err: MappedErrors) -> HttpResponse {
    let code_string = err.code().to_string();

    let error_maps = vec![
        (MYC00001, HttpResponse::InternalServerError()),
        (MYC00002, HttpResponse::Conflict()),
        (MYC00003, HttpResponse::Conflict()),
        (MYC00004, HttpResponse::InternalServerError()),
        (MYC00005, HttpResponse::BadRequest()),
        (MYC00006, HttpResponse::BadRequest()),
        (MYC00007, HttpResponse::InternalServerError()),
        (MYC00008, HttpResponse::BadRequest()),
        (MYC00009, HttpResponse::BadRequest()),
        (MYC00010, HttpResponse::InternalServerError()),
        (MYC00011, HttpResponse::BadRequest()),
        (MYC00012, HttpResponse::InternalServerError()),
        (MYC00013, HttpResponse::BadRequest()),
        (MYC00014, HttpResponse::Conflict()),
        (MYC00015, HttpResponse::Conflict()),
        (MYC00016, HttpResponse::BadRequest()),
        (MYC00017, HttpResponse::Conflict()),
        (MYC00018, HttpResponse::Conflict()),
        (MYC00019, HttpResponse::Forbidden()),
        (MYC00020, HttpResponse::Forbidden()),
        (MYC00021, HttpResponse::BadRequest()),
        (MYC00022, HttpResponse::BadRequest()),
        (MYC00023, HttpResponse::BadRequest()),
    ];

    for (code, mut response) in error_maps {
        if err.is_in(vec![code]) {
            error!("Error: {err}");

            return response.json(
                HttpJsonResponse::new_message(err.to_string())
                    .with_code(code_string),
            );
        }
    }

    error!("Unhandled error: {err}");

    HttpResponse::InternalServerError().json(
        HttpJsonResponse::new_message("Unexpcted internal error")
            .with_code(code_string),
    )
}
