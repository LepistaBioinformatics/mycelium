use super::{ProfileJwksUrl, TrustedProfileKeys};
use crate::settings::PROFILE_JWKS_EMPTY_RETRY_SECONDS;

use futures::{future::LocalBoxFuture, lock::Mutex};
use mycelium_base::utils::errors::MappedErrors;
use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    sync::{Arc, PoisonError, RwLock},
    time::{Duration, Instant},
};

/// A key set read from the gateway, with how long it may be cached
#[derive(Clone, Debug)]
pub struct FetchedProfileJwks {
    pub keys: TrustedProfileKeys,
    pub max_age: Duration,
}

/// How the store downloads the JWKS (HTTP in production, fakes in tests)
pub type ProfileJwksFetcher = Arc<
    dyn Fn(
            ProfileJwksUrl,
        )
            -> LocalBoxFuture<'static, Result<FetchedProfileJwks, MappedErrors>>
        + Send
        + Sync,
>;

#[derive(Default)]
struct CachedKeys {
    keys: Arc<TrustedProfileKeys>,
    generation: u64,
    fresh_until: Option<Instant>,
    last_attempt: Option<Instant>,
}

/// The gateway public keys cached by a downstream verifier
///
/// - Fetched before serving (`prefetch`) or lazily on the first request.
/// - Refreshed when `max-age` expires; a failed refresh keeps the last valid
///   set and is logged, so a gateway outage never drops trusted keys.
/// - Refetched once when a signature names an unknown `kid`, at most once
///   per cooldown, and single-flight: concurrent requests share one fetch.
///
pub struct ProfileKeyStore {
    url: ProfileJwksUrl,
    fetcher: ProfileJwksFetcher,
    cooldown: Duration,
    cache: RwLock<CachedKeys>,
    refresh: Mutex<()>,
}

impl ProfileKeyStore {
    pub fn new(
        url: ProfileJwksUrl,
        fetcher: ProfileJwksFetcher,
        cooldown: Duration,
    ) -> Self {
        Self {
            url,
            fetcher,
            cooldown,
            cache: RwLock::new(CachedKeys::default()),
            refresh: Mutex::new(()),
        }
    }

    /// The trusted keys now, and the generation they belong to
    pub fn current(&self) -> (Arc<TrustedProfileKeys>, u64) {
        self.read(|cache| (cache.keys.clone(), cache.generation))
    }

    /// Fetch the set now; any failure is returned (startup must abort)
    #[tracing::instrument(name = "profile_key_store_prefetch", skip_all)]
    pub async fn prefetch(&self) -> Result<(), MappedErrors> {
        let _guard = self.refresh.lock().await;

        self.fetch_and_store().await
    }

    /// Refresh when the cached set expired (or was never fetched)
    #[tracing::instrument(name = "profile_key_store_refresh", skip_all)]
    pub async fn refresh_if_stale(&self) {
        let (generation, is_stale) = self.read(|cache| {
            let is_stale = cache
                .fresh_until
                .map_or(true, |until| Instant::now() >= until);
            (cache.generation, is_stale)
        });

        if is_stale {
            self.refresh_from(generation).await;
        }
    }

    /// Refetch after an unknown `kid`; `true` when newer keys are available
    #[tracing::instrument(name = "profile_key_store_unknown_kid", skip_all)]
    pub async fn refresh_after_unknown_key(
        &self,
        seen_generation: u64,
    ) -> bool {
        self.refresh_from(seen_generation).await
    }

    /// Single-flight refresh: waiters reuse a fetch that completed meanwhile
    async fn refresh_from(&self, seen_generation: u64) -> bool {
        let _guard = self.refresh.lock().await;
        let (generation, last_attempt) =
            self.read(|cache| (cache.generation, cache.last_attempt));

        if generation != seen_generation {
            return true;
        }

        let wait = self.retry_interval(generation);
        if last_attempt.is_some_and(|at| at.elapsed() < wait) {
            return false;
        }

        self.fetch_and_store()
            .await
            .map_err(log_refresh_failure)
            .is_ok()
    }

    async fn fetch_and_store(&self) -> Result<(), MappedErrors> {
        self.update(|cache| cache.last_attempt = Some(Instant::now()));
        let fetched = (self.fetcher)(self.url.clone()).await?;

        self.update(|cache| {
            cache.keys = Arc::new(fetched.keys);
            cache.generation += 1;
            cache.fresh_until = Some(Instant::now() + fetched.max_age);
        });

        Ok(())
    }

    /// The cooldown, shortened while no key set was ever stored, so a
    /// service that started before the gateway recovers within seconds
    fn retry_interval(&self, generation: u64) -> Duration {
        if generation > 0 {
            return self.cooldown;
        }

        self.cooldown
            .min(Duration::from_secs(PROFILE_JWKS_EMPTY_RETRY_SECONDS))
    }

    fn read<T>(&self, view: impl FnOnce(&CachedKeys) -> T) -> T {
        view(&self.cache.read().unwrap_or_else(PoisonError::into_inner))
    }

    fn update(&self, change: impl FnOnce(&mut CachedKeys)) {
        let mut cache =
            self.cache.write().unwrap_or_else(PoisonError::into_inner);

        change(&mut cache);
    }
}

fn log_refresh_failure(error: MappedErrors) {
    tracing::warn!(
        event = "profile_signature.jwks",
        outcome = "refresh_failed",
        reason = %error.msg(),
        "keeping the last valid profile JWKS"
    );
}

impl Debug for ProfileKeyStore {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        formatter
            .debug_struct("ProfileKeyStore")
            .field("url", &self.url.as_str())
            .field("cooldown", &self.cooldown)
            .finish()
    }
}
