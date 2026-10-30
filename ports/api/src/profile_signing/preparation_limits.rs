use std::time::Duration;

/// Internal boot budgets; never configured by an untrusted request.
#[derive(Clone, Copy)]
pub(super) struct PreparationLimits {
    pub per_key: Duration,
    pub total: Duration,
}

impl Default for PreparationLimits {
    fn default() -> Self {
        Self {
            per_key: Duration::from_secs(10),
            total: Duration::from_secs(60),
        }
    }
}
