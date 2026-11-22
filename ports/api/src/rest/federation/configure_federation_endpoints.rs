use super::{
    handshake_endpoints, link_endpoints, link_pages, resolution_endpoints,
    tombstone_endpoints,
};
use crate::{
    federation::{
        AssertionVerifier, PreparedFederation, SignedPeerGrantResolver,
        SignedPeerLinkNotifier, SignedPeerPoster,
    },
    models::active_backend_modules::SqlAppModule,
};

use actix_web::{
    body::MessageBody,
    dev::{ServiceFactory, ServiceRequest, ServiceResponse},
    web, App, Error,
};
use myc_core::domain::entities::{
    FederationPeerRead, FederationPeerWrite, PeerClient, PeerGrantResolver,
    PeerLinkNotifier, RequestNonceRegistration,
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

    let resolver = grant_resolver(
        prepared,
        peer_client.clone(),
        peer_read.clone(),
        peer_write.clone(),
    );

    app.app_data(web::Data::from(peer_client))
        .app_data(web::Data::from(peer_read))
        .app_data(web::Data::from(peer_write))
        .app_data(web::Data::from(nonces))
        .app_data(web::Data::from(notifier))
        .app_data(web::Data::from(resolver))
        .configure(handshake_endpoints::configure)
        .configure(link_endpoints::configure)
        .configure(link_pages::configure)
        .configure(resolution_endpoints::configure)
        .configure(tombstone_endpoints::configure)
}

/// How this instance asks its peers for a linked subject's grants (FED-11)
fn grant_resolver(
    prepared: &PreparedFederation,
    peer_client: Arc<dyn PeerClient>,
    peer_read: Arc<dyn FederationPeerRead>,
    peer_write: Arc<dyn FederationPeerWrite>,
) -> Arc<dyn PeerGrantResolver> {
    Arc::new(SignedPeerGrantResolver {
        poster: SignedPeerPoster {
            prepared: prepared.clone(),
            peer_client: peer_client.clone(),
        },
        verifier: AssertionVerifier {
            prepared: prepared.clone(),
            peer_client,
            peer_read,
            peer_write,
        },
    })
}
