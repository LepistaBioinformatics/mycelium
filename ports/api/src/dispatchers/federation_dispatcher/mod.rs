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
use tokio::{sync::watch, task::JoinHandle, time::MissedTickBehavior};

/// Time between two federation health rounds
const ROUND_INTERVAL: Duration = Duration::from_secs(30);

/// Time between two pushes of queued tombstones (FED-22: within seconds)
const REVOCATION_INTERVAL: Duration = Duration::from_secs(1);

/// The running federation dispatcher; keep it and call `shutdown` so its
/// tasks are awaited, never dropped (R-8)
pub(crate) struct FederationDispatcher {
    stop: watch::Sender<bool>,
    handles: [JoinHandle<()>; 2],
}

impl FederationDispatcher {
    /// Signal the loops to stop and wait for the rounds in flight to finish
    pub(crate) async fn shutdown(self) {
        let _ = self.stop.send(true);

        for handle in self.handles {
            if let Err(err) = handle.await {
                tracing::error!(%err, "federation dispatcher ended abnormally");
            }
        }
    }
}

/// Start the supervised loops: peer health rounds, and tombstone pushes on
/// their own task so a slow health round never holds a revocation back
#[tracing::instrument(name = "spawn_federation_dispatcher", skip_all)]
pub(crate) fn spawn_federation_dispatcher(
    ports: FederationRoundPorts,
    revocations: RevocationRound,
    local_kinds: Vec<String>,
    max_failures_before_open: u32,
) -> FederationDispatcher {
    let (stop, mut stopped) = watch::channel(false);
    let pushes = spawn_revocation_pushes(revocations, stop.subscribe());
    let rounds = tokio::spawn(async move {
        let mut interval = tokio::time::interval(ROUND_INTERVAL);

        loop {
            tokio::select! {
                _ = stopped.changed() => return,
                _ = interval.tick() => run_federation_round(
                    &ports,
                    &local_kinds,
                    max_failures_before_open,
                    Local::now(),
                ).await,
            }
        }
    });

    FederationDispatcher {
        stop,
        handles: [rounds, pushes],
    }
}

/// Push queued tombstones every second until stopped (FED-22)
fn spawn_revocation_pushes(
    revocations: RevocationRound,
    mut stopped: watch::Receiver<bool>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(REVOCATION_INTERVAL);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = stopped.changed() => return,
                _ = interval.tick() => revocations.run().await,
            }
        }
    })
}
