// All actions listed below should ve performed by:
//
// - Platform Staffs
// - Platform Managers
// - Tenant Owner
//
// The above cited roles should be able to manage the tenant to perform the
// following functions:
//
// - Create tenant metadata;
// - Delete tenant metadata;
//

mod create_tenant_meta;
mod delete_tenant_meta;

pub use create_tenant_meta::*;
pub use delete_tenant_meta::*;
