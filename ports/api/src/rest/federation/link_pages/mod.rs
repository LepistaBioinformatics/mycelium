mod approve_link;
mod authorize_link;
mod consumer_link_pages;
mod enabled_federation;
mod link_callback;
mod link_request_token;
mod render_link_page;

use actix_web::web;
use myc_http_tools::settings::{
    FEDERATION_LINK_APPROVE_PATH, FEDERATION_LINK_AUTHORIZE_PATH,
    FEDERATION_LINK_CALLBACK_PATH, FEDERATION_LINK_DONE_PATH,
    FEDERATION_LINK_START_PATH,
};

/// The browser-facing linking pages (FED-42 … FED-46). Not peer-signed: the
/// consumer pages are anchored by the single-use initiation URL, the issuer
/// pages by the signed state, the hosted login and the consent.
pub(crate) fn configure(config: &mut web::ServiceConfig) {
    config
        .route(
            FEDERATION_LINK_START_PATH,
            web::get().to(consumer_link_pages::show_link_start),
        )
        .route(
            FEDERATION_LINK_START_PATH,
            web::post().to(consumer_link_pages::continue_link_start),
        )
        .route(
            FEDERATION_LINK_DONE_PATH,
            web::get().to(consumer_link_pages::show_link_done),
        )
        .route(
            FEDERATION_LINK_AUTHORIZE_PATH,
            web::get().to(authorize_link::authorize_link),
        )
        .route(
            FEDERATION_LINK_CALLBACK_PATH,
            web::get().to(link_callback::link_callback),
        )
        .route(
            FEDERATION_LINK_APPROVE_PATH,
            web::post().to(approve_link::approve_link),
        );
}
