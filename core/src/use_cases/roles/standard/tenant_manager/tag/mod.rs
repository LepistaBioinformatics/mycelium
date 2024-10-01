// Only tenant managers and owners should manage tags for a given tenant. Then,
// the accounts with the above cited roles should be able to perform the
// following functions:
//
// - Create tags;
// - Update tags;
// - Delete tags.
//

mod delete_tag;
mod register_tag;
mod update_tag;

pub use delete_tag::*;
pub use register_tag::*;
pub use update_tag::*;
