// All actions listed below should ve performed by:
//
// - Platform Staffs
// - Platform Managers
// - Tenant Owner
//
// The above cited roles should be able to manage the tenant to perform the
// following functions:
//
// - Create management accounts;
// - Delete tenant manager accounts;

mod create_management_account;
mod delete_tenant_manager_account;

pub use create_management_account::*;
pub use delete_tenant_manager_account::*;
