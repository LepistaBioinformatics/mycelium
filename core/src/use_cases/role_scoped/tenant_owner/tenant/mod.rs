// All actions listed below should ve performed by:
//
// - Platform Staffs
// - Platform Managers
// - Tenant Owner
//
// Tenant Owner should be able to perform the following actions:
//
// - Update Tenant Name and Description
// - Update Tenant Archiving Status
// - Update Tenant Trashing Status
// - Update Tenant Verifying Status
//

mod update_tenant_archiving_status;
mod update_tenant_name_and_description;
mod update_tenant_status;
mod update_tenant_trashing_status;
mod update_tenant_verifying_status;

pub use update_tenant_archiving_status::*;
pub use update_tenant_name_and_description::*;
pub use update_tenant_trashing_status::*;
pub use update_tenant_verifying_status::*;
