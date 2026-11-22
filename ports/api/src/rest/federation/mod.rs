mod configure_federation_endpoints;
pub(crate) mod handshake_endpoints;
mod link_endpoints;
mod link_pages;
mod refresh_peer_keys;
mod resolution_endpoints;
mod signed_peer_request;
mod tombstone_endpoints;
pub(crate) mod verify_with_peer_keys;
mod vocabulary_endpoints;

#[cfg(test)]
pub(crate) mod peer_request_test_doubles;

pub(crate) use configure_federation_endpoints::*;
pub(crate) use signed_peer_request::*;
