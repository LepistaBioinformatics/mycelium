// All actions listed below should ve performed by:
//
// - Tenant Owner
// - Platform Managers
// - Platform Staffs
//

mod update_tenant_archiving_status;
mod update_tenant_name_and_description;
mod update_tenant_trashing_status;
mod update_tenant_verifying_status;

pub use update_tenant_archiving_status::*;
pub use update_tenant_name_and_description::*;
pub use update_tenant_trashing_status::*;
pub use update_tenant_verifying_status::*;
