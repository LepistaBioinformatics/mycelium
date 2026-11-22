use myc_core::{
    domain::entities::{
        AssertionStoreWrite, FederationPeerRead, InstanceSettingsFetching,
        PeerTombstoneSender, RevocationOutbox,
    },
    use_cases::federation::{
        deliver_revocations, is_federation_enabled, RevocationDeliveryPorts,
    },
};
use std::sync::Arc;

/// What pushing queued tombstones uses (FED-24)
#[derive(Clone)]
pub(crate) struct RevocationRound {
    pub(crate) outbox: Arc<dyn RevocationOutbox>,
    pub(crate) peers: Arc<dyn FederationPeerRead>,
    pub(crate) sender: Arc<dyn PeerTombstoneSender>,
    pub(crate) settings: Arc<dyn InstanceSettingsFetching>,
}

impl RevocationRound {
    /// Push every due tombstone once; skipped while the kill-switch is off
    #[tracing::instrument(name = "run_revocation_round", skip_all)]
    pub(crate) async fn run(&self) {
        if !matches!(
            is_federation_enabled(Box::new(self.settings.as_ref())).await,
            Ok(true)
        ) {
            return;
        }

        let ports = RevocationDeliveryPorts {
            outbox: self.outbox.as_ref(),
            peers: self.peers.as_ref(),
            sender: self.sender.as_ref(),
        };

        if let Err(err) = deliver_revocations(ports).await {
            tracing::error!(%err, "revocation delivery failed");
        }
    }
}

/// Drop every stored assertion before serving: this instance cannot know
/// which tombstones it missed while it was down, so it asks each issuer again
/// before serving any grant (FED-24)
#[tracing::instrument(name = "forget_stored_assertions", skip_all)]
pub(crate) async fn forget_stored_assertions(
    assertions: &dyn AssertionStoreWrite,
) {
    match assertions.forget_all().await {
        Ok(forgotten) => {
            tracing::info!(forgotten, "stored assertions dropped at boot")
        }
        Err(err) => tracing::error!(%err, "stored assertions not dropped"),
    }
}
