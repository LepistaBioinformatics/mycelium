// Only tenant owners should manage tenant tags. Then, the tenant owner
// should be able to manage the tenant to perform the following functions:
//
// - Create tenant tags;
// - Update tenant tags;
// - Delete tenant tags;
//

mod create_tenant_tag;
mod delete_tenant_tag;
mod update_tenant_tag;

pub use create_tenant_tag::*;
pub use delete_tenant_tag::*;
pub use update_tenant_tag::*;
