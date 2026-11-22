use crate::domain::entities::{
    FederationPeerRead, PeerTombstoneSender, RevocationOutbox,
};

/// What delivering queued revocations reads, calls and writes
#[derive(Clone, Copy)]
pub struct RevocationDeliveryPorts<'a> {
    pub outbox: &'a dyn RevocationOutbox,
    pub peers: &'a dyn FederationPeerRead,
    pub sender: &'a dyn PeerTombstoneSender,
}
