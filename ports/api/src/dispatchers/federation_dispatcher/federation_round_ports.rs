use super::ExpiredRowsPurge;

use myc_core::domain::entities::{
    FederationPeerRead, FederationPeerWrite, InstanceSettingsFetching,
    PeerClient,
};
use std::sync::Arc;

/// What one federation health round reads and writes
#[derive(Clone)]
pub(crate) struct FederationRoundPorts {
    pub(crate) peer_read: Arc<dyn FederationPeerRead>,
    pub(crate) peer_write: Arc<dyn FederationPeerWrite>,
    pub(crate) purges: Vec<Arc<dyn ExpiredRowsPurge>>,
    pub(crate) settings: Arc<dyn InstanceSettingsFetching>,
    pub(crate) peer_client: Arc<dyn PeerClient>,
}
