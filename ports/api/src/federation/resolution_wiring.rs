use super::PreparedFederation;
use crate::models::active_backend_modules::SqlAppModule;

use myc_core::{
    domain::entities::PeerGrantResolver,
    use_cases::federation::{
        ResolutionGuardPorts, ResolutionLimits, ResolutionPeerPorts,
        ResolutionPorts, ResolutionStorePorts,
    },
};
use shaku::HasComponent;
use std::time::Duration;

/// The ports a resolution uses, all from the SQL module but the resolver
pub(crate) fn resolution_ports<'a>(
    app_module: &'a SqlAppModule,
    resolver: &'a dyn PeerGrantResolver,
) -> ResolutionPorts<'a> {
    ResolutionPorts {
        links: app_module.resolve_ref(),
        peers: ResolutionPeerPorts {
            read: app_module.resolve_ref(),
            write: app_module.resolve_ref(),
            resolver,
        },
        store: ResolutionStorePorts {
            read: app_module.resolve_ref(),
            write: app_module.resolve_ref(),
        },
        guard: ResolutionGuardPorts {
            probe: app_module.resolve_ref(),
            claims: app_module.resolve_ref(),
        },
        audit: app_module.resolve_ref(),
    }
}

/// The resolution bounds of `[core.federation]`
pub(crate) fn resolution_limits(
    prepared: &PreparedFederation,
) -> ResolutionLimits {
    ResolutionLimits {
        deadline: Duration::from_millis(prepared.config.resolve_deadline_ms),
        max_failures_before_open: prepared.config.max_failures_before_open,
        assertion_ttl: Duration::from_secs(prepared.config.assertion_ttl_secs),
        assertion_refresh: Duration::from_secs(
            prepared.config.assertion_refresh_secs,
        ),
    }
}
