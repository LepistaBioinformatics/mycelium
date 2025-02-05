/// KV Artifacts are key-value pairs that are stored in the database
///
/// The kv artifact is not part of the core DTOs, but from the base lib.
///
mod kv_artifact_read;
mod kv_artifact_write;

pub use kv_artifact_read::*;
pub use kv_artifact_write::*;
