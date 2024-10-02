// All actions listed below should ve performed by:
//
// - Platform Staffs
// - Platform Managers
// - Tenant Owner
//
// The above cited roles should be able to manage the tenant to perform the
// following functions:
//
// - Guest new owners as the tenant owners;
// - Revoking a tenant owner;
//

mod guest_tenant_owner;
mod revoke_tenant_owner;

pub use guest_tenant_owner::*;
pub use revoke_tenant_owner::*;
