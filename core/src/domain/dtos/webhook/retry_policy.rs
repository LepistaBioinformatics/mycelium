/// How the webhook queue spaces retries and how long a claim survives
///
/// Carried as an argument of `WebHookFetching::fetch_execution_event` rather
/// than injected into the repositories. `ports/api/src/main.rs` builds the
/// repository graph at two separate sites (full mode and postgres-only); a
/// `#[shaku(default)]` field missed at one of them would silently resolve to
/// zero, and a zero window makes the reclaim predicate always true -- the
/// dedup fix would simply not apply in that mode, without a compile error.
/// Passing the policy in keeps the wiring in one place.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WebHookRetryPolicy {
    /// The delay before the first retry, in seconds
    pub retry_base_in_secs: u64,

    /// The ceiling on the exponential delay, in seconds
    pub retry_cap_in_secs: u64,

    /// How long a claimed event stays un-reclaimable, in seconds
    ///
    /// This is crash recovery, not back-off: it only governs how long an event
    /// left `Processing` by a pod that died stays invisible to the others. It
    /// MUST exceed the worst-case wall-clock of one whole claimed batch, since
    /// the batch is marked up front and dispatched sequentially -- otherwise a
    /// live-but-slow pod has its un-dispatched rows reclaimed and double-sent.
    ///
    pub visibility_timeout_in_secs: i64,
}

impl WebHookRetryPolicy {
    pub fn new(
        retry_base_in_secs: u64,
        retry_cap_in_secs: u64,
        visibility_timeout_in_secs: i64,
    ) -> Self {
        Self {
            retry_base_in_secs,
            retry_cap_in_secs,
            visibility_timeout_in_secs,
        }
    }

    /// The delay owed after `attempts` failed attempts
    ///
    /// `min(base * 2^attempts, cap)`. Both the shift and the multiplication
    /// saturate, so a large `maxAttempts` yields the cap rather than wrapping
    /// around to a delay of nearly zero.
    ///
    pub fn backoff_in_secs(&self, attempts: u32) -> u64 {
        let factor = 1u64.checked_shl(attempts).unwrap_or(u64::MAX);

        self.retry_base_in_secs
            .saturating_mul(factor)
            .min(self.retry_cap_in_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> WebHookRetryPolicy {
        WebHookRetryPolicy::new(30, 3600, 900)
    }

    #[test]
    fn backoff_doubles_with_each_attempt() {
        let policy = policy();

        assert_eq!(policy.backoff_in_secs(0), 30);
        assert_eq!(policy.backoff_in_secs(1), 60);
        assert_eq!(policy.backoff_in_secs(2), 120);
        assert_eq!(policy.backoff_in_secs(3), 240);
        assert_eq!(policy.backoff_in_secs(4), 480);
    }

    #[test]
    fn backoff_stops_at_the_cap() {
        let policy = policy();

        assert_eq!(policy.backoff_in_secs(7), 3600);
        assert_eq!(policy.backoff_in_secs(8), 3600);
    }

    #[test]
    fn backoff_saturates_instead_of_wrapping() {
        let policy = policy();

        // `1u64 << 64` is undefined-shift territory; a wrap here would hand
        // back a near-zero delay and turn the back-off into a hot loop.
        assert_eq!(policy.backoff_in_secs(64), 3600);
        assert_eq!(policy.backoff_in_secs(u32::MAX), 3600);
    }

    #[test]
    fn a_cap_below_the_base_still_wins() {
        let policy = WebHookRetryPolicy::new(300, 60, 900);

        assert_eq!(policy.backoff_in_secs(0), 60);
    }
}
