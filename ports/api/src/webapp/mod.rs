//! The webapp build served under `/_app/` (`webapp` feature)

#[cfg(feature = "webapp")]
mod resolve_webapp_file;
#[cfg(feature = "webapp")]
mod serve_webapp_file;
mod served_webapp;
#[cfg(feature = "webapp")]
mod webapp_response_headers;

pub(crate) use served_webapp::ServedWebapp;

use actix_web::web;

/// Mount the webapp routes when a build is served
///
/// Mounts nothing otherwise, so `/_app/*` keeps going to the downstream router
/// (WEBAPP-08).
///
pub(crate) fn configure_webapp(
    config: &mut web::ServiceConfig,
    served: Option<ServedWebapp>,
) {
    let Some(served) = served else {
        return;
    };

    #[cfg(feature = "webapp")]
    serve_webapp_file::configure(config, served);

    #[cfg(not(feature = "webapp"))]
    let _ = (config, served);
}
