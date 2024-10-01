// Only tenant owners should menage tenant metadata. Then, the tenant owner
// should be able to manage the tenant to perform the following functions:
//
// - Create tenant metadata;
// - Update tenant metadata;
// - Delete tenant metadata;
//

mod create_tenant_meta;
mod delete_tenant_meta;
mod update_tenant_meta;

pub use create_tenant_meta::*;
pub use delete_tenant_meta::*;
pub use update_tenant_meta::*;
