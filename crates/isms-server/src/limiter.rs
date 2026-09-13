//! Token-bucket rate limiting (TDD 10.2, T15). Per-citizen limits come from
//! the society's published `Capabilities.rate_limit`; per-IP limits guard the
//! auth endpoints (TDD 14).

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    last: Instant,
}

/// Keyed buckets; each key refills at its own rate.
#[derive(Debug, Default)]
pub struct RateLimiter {
    buckets: Mutex<HashMap<String, Bucket>>,
}

impl RateLimiter {
    /// Take one token for `key`; `false` when the bucket is empty.
    pub fn check(&self, key: &str, per_second: f64, burst: f64) -> bool {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().expect("limiter mutex");
        let bucket = buckets.entry(key.to_owned()).or_insert(Bucket {
            tokens: burst,
            last: now,
        });
        let elapsed = now.duration_since(bucket.last).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * per_second).min(burst);
        bucket.last = now;
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn burst_then_refuse() {
        let l = RateLimiter::default();
        for _ in 0..5 {
            assert!(l.check("a", 1.0, 5.0));
        }
        assert!(!l.check("a", 1.0, 5.0));
        assert!(l.check("b", 1.0, 5.0));
    }
}
