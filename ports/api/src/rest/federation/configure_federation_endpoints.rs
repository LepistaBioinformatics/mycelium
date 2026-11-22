use super::{handshake_endpoints, link_endpoints, link_pages};
use crate::{
    federation::{PreparedFederation, SignedPeerLinkNotifier},
    models::active_backend_modules::SqlAppModule,
};

use actix_web::{
    body::MessageBody,
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    web, App, Error,
};
use myc_core::domain::entities::{
    FederationPeerRead, FederationPeerWrite, PeerClient, PeerLinkNotifier,
    RequestNonceRegistration,
};
use shaku::HasComponent;
use std::sync::Arc;

/// Mount the `/federation/*` endpoints (peer-authenticated calls and the
/// browser linking pages) with the ports their extractor and handlers resolve
pub(crate) fn configure_federation_endpoints<T, B>(
    app: App<T>,
    sql_module: &Arc<SqlAppModule>,
    prepared: &PreparedFederation,
    peer_client: Arc<dyn PeerClient>,
) -> App<T>
where
    B: MessageBody,
    T: ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<B>,
        Error = Error,
        InitError = (),
    >,
{
    let peer_read: Arc<dyn FederationPeerRead> = sql_module.resolve();
    let peer_write: Arc<dyn FederationPeerWrite> = sql_module.resolve();
    let nonces: Arc<dyn RequestNonceRegistration> = sql_module.resolve();
    let notifier: Arc<dyn PeerLinkNotifier> = Arc::new(
        SignedPeerLinkNotifier::new(prepared.clone(), peer_client.clone()),
    );

    app.app_data(web::Data::from(peer_client))
        .app_data(web::Data::from(peer_read))
        .app_data(web::Data::from(peer_write))
        .app_data(web::Data::from(nonces))
        .app_data(web::Data::from(notifier))
        .configure(handshake_endpoints::configure)
        .configure(link_endpoints::configure)
        .configure(link_pages::configure)
}
