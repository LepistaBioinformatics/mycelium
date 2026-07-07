use std::{sync::Arc, time::Duration};

use moka::future::Cache;
use moka::Expiry;
use shaku::{Component, Interface};

pub type ArtifactCache = Cache<String, (String, Duration)>;

pub trait MokaCacheProvider: Interface + Send + Sync {
    fn get_cache(&self) -> Arc<ArtifactCache>;
}

#[derive(Component)]
#[shaku(interface = MokaCacheProvider)]
#[derive(Clone)]
pub struct MokaCacheProviderImpl {
    pub(crate) cache: Arc<ArtifactCache>,
}

impl MokaCacheProvider for MokaCacheProviderImpl {
    fn get_cache(&self) -> Arc<ArtifactCache> {
        self.cache.clone()
    }
}

impl MokaCacheProviderImpl {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(
                Cache::builder().expire_after(PerKeyExpiry).build(),
            ),
        }
    }
}

impl Default for MokaCacheProviderImpl {
    fn default() -> Self {
        Self::new()
    }
}

struct PerKeyExpiry;

impl Expiry<String, (String, Duration)> for PerKeyExpiry {
    fn expire_after_create(
        &self,
        _key: &String,
        value: &(String, Duration),
        _created_at: std::time::Instant,
    ) -> Option<Duration> {
        Some(value.1)
    }
}
