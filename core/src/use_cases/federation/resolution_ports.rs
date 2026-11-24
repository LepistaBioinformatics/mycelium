use crate::domain::entities::{
    AssertionStoreRead, AssertionStoreWrite, FederationPeerRead,
    FederationPeerWrite, IdentityLinkRead, LocalNamespaceProbe,
    NamespaceClaimRegistration, PeerGrantResolver,
    ResourceAuditLogRegistration,
};

use std::time::Duration;

/// The peers and how to reach them
pub struct ResolutionPeerPorts<'a> {
    pub read: &'a dyn FederationPeerRead,
    pub write: &'a dyn FederationPeerWrite,
    pub resolver: &'a dyn PeerGrantResolver,
}

/// The assertion store (D-6)
pub struct ResolutionStorePorts<'a> {
    pub read: &'a dyn AssertionStoreRead,
    pub write: &'a dyn AssertionStoreWrite,
}

/// The ID-collision guard (I-1)
pub struct ResolutionGuardPorts<'a> {
    pub probe: &'a dyn LocalNamespaceProbe,
    pub claims: &'a dyn NamespaceClaimRegistration,
}

/// Everything a federated resolution reads and writes
pub struct ResolutionPorts<'a> {
    pub links: &'a dyn IdentityLinkRead,
    pub peers: ResolutionPeerPorts<'a>,
    pub store: ResolutionStorePorts<'a>,
    pub guard: ResolutionGuardPorts<'a>,
    pub audit: &'a dyn ResourceAuditLogRegistration,
}

/// The bounds of a resolution (FED-13, FED-21)
#[derive(Clone, Copy, Debug)]
pub struct ResolutionLimits {
    /// Hard total deadline; the per-peer timeout is the HTTP client's
    pub deadline: Duration,
    pub max_failures_before_open: u32,
    /// The longest a received assertion is kept, whatever its `exp` (D-1)
    pub assertion_ttl: Duration,
    /// Age after which a stored assertion is refreshed (FED-17)
    pub assertion_refresh: Duration,
}
