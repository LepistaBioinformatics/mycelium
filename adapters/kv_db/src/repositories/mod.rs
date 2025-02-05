use myc_adapters_shared_lib::models::SharedClientImpl;
use shaku::module;

mod kv_artifact_read;
mod kv_artifact_write;

pub(crate) use kv_artifact_read::*;
pub(crate) use kv_artifact_write::*;

module! {
    pub KVAppModule {
        components = [
            SharedClientImpl,
            KVArtifactReadRepository,
            KVArtifactWriteRepository,
        ],
        providers = []
    }
}
