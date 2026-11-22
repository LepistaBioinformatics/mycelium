mod expired_rows_purge;
mod federation_round_ports;
mod revocation_round;
mod run_federation_round;

pub(crate) use expired_rows_purge::*;
pub(crate) use federation_round_ports::*;
pub(crate) use revocation_round::*;
pub(crate) use run_federation_round::*;

use chrono::Local;
use std::time::Duration;
use tokio::{sync::watch, task::JoinHandle};

/// Time between two federation health rounds
const ROUND_INTERVAL: Duration = Duration::from_secs(30);

/// Time between two pushes of queued tombstones (FED-22: within seconds)
const REVOCATION_INTERVAL: Duration = Duration::from_secs(1);

/// The running federation dispatcher; keep it and call `shutdown` so its
/// task is awaited, never dropped (R-8)
pub(crate) struct FederationDispatcher {
    stop: watch::Sender<bool>,
    handle: JoinHandle<()>,
}

impl FederationDispatcher {
    /// Signal the loop to stop and wait for the round in flight to finish
    pub(crate) async fn shutdown(self) {
        let _ = self.stop.send(true);

        if let Err(err) = self.handle.await {
            tracing::error!(%err, "federation dispatcher ended abnormally");
        }
    }
}

/// Start the supervised loop of federation health rounds and tombstone
/// pushes; the stored assertions are dropped first (FED-24)
#[tracing::instrument(name = "spawn_federation_dispatcher", skip_all)]
pub(crate) fn spawn_federation_dispatcher(
    ports: FederationRoundPorts,
    revocations: RevocationRound,
    local_kinds: Vec<String>,
    max_failures_before_open: u32,
) -> FederationDispatcher {
    let (stop, mut stopped) = watch::channel(false);
    let handle = tokio::spawn(async move {
        revocations.forget_stored_assertions().await;
        let mut rounds = tokio::time::interval(ROUND_INTERVAL);
        let mut pushes = tokio::time::interval(REVOCATION_INTERVAL);

        loop {
            tokio::select! {
                _ = stopped.changed() => return,
                _ = pushes.tick() => revocations.run().await,
                _ = rounds.tick() => run_federation_round(
                    &ports,
                    &local_kinds,
                    max_failures_before_open,
                    Local::now(),
                ).await,
            }
        }
    });

    FederationDispatcher { stop, handle }
}
