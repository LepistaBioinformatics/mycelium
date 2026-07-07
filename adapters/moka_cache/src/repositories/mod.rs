use shaku::module;

use crate::config::MokaCacheProviderImpl;

mod kv_artifact_read;
mod kv_artifact_write;

pub(crate) use kv_artifact_read::*;
pub(crate) use kv_artifact_write::*;

module! {
    pub KVAppModule {
        components = [
            MokaCacheProviderImpl,
            KVArtifactReadRepository,
            KVArtifactWriteRepository,
        ],
        providers = []
    }
}
