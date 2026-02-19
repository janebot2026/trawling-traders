use chrono::Utc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Tracks API call outcomes to support health reporting without live API calls.
///
/// All fields use `AtomicU64` so `HealthTracker` can be shared across threads
/// without a `Mutex`. Counters are lifetime totals (not rolling 24h windows);
/// the derived `success_rate` reflects the full observed lifetime of the client.
pub struct HealthTracker {
    /// Milliseconds since UNIX epoch of last successful request (0 = never)
    pub last_success_ms: AtomicU64,
    /// Milliseconds since UNIX epoch of last failed request (0 = never)
    pub last_failure_ms: AtomicU64,
    /// Lifetime count of successful requests
    pub success_count: AtomicU64,
    /// Lifetime count of failed requests
    pub failure_count: AtomicU64,
    /// Latency of the most recent successful request in milliseconds
    pub last_latency_ms: AtomicU64,
}

impl HealthTracker {
    pub fn new() -> Self {
        Self {
            last_success_ms: AtomicU64::new(0),
            last_failure_ms: AtomicU64::new(0),
            success_count: AtomicU64::new(0),
            failure_count: AtomicU64::new(0),
            last_latency_ms: AtomicU64::new(0),
        }
    }

    /// Record a successful request with its observed latency.
    pub fn record_success(&self, latency_ms: u64) {
        let now_ms = Utc::now().timestamp_millis() as u64;
        self.last_success_ms.store(now_ms, Ordering::Relaxed);
        self.last_latency_ms.store(latency_ms, Ordering::Relaxed);
        self.success_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed request.
    pub fn record_failure(&self) {
        let now_ms = Utc::now().timestamp_millis() as u64;
        self.last_failure_ms.store(now_ms, Ordering::Relaxed);
        self.failure_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns `true` if the source has had at least one success and the most
    /// recent outcome was a success (last success timestamp > last failure timestamp).
    pub fn is_healthy(&self) -> bool {
        let last_success = self.last_success_ms.load(Ordering::Relaxed);
        let last_failure = self.last_failure_ms.load(Ordering::Relaxed);
        last_success > 0 && (last_failure == 0 || last_success > last_failure)
    }

    /// Lifetime success rate in `[0.0, 1.0]`. Returns `1.0` when no requests
    /// have been made yet (optimistic default).
    pub fn success_rate(&self) -> f64 {
        let successes = self.success_count.load(Ordering::Relaxed);
        let failures = self.failure_count.load(Ordering::Relaxed);
        let total = successes + failures;
        if total == 0 {
            return 1.0;
        }
        successes as f64 / total as f64
    }
}

impl Default for HealthTracker {
    fn default() -> Self {
        Self::new()
    }
}
