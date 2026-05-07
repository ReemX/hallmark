//! Cross-source duplicate suppression — REQ DETECT-07.
//!
//! When multiple source adapters observe the same logical unlock (e.g. a user runs
//! a legitimate Steam game with Goldberg also active and watching), each adapter
//! independently emits a `RawUnlockEvent`. This stage collapses them.
//!
//! # Layering
//!
//! Phase 1's pipeline has THREE deduplication layers:
//!
//! 1. **notify-debouncer-full** — collapses bursts of FS events on the same path
//!    within 500ms (REQ DETECT-06 layer 1).
//! 2. **Per-adapter SHA-256 content hash** — collapses identical re-writes within
//!    one adapter (REQ DETECT-06 layer 2).
//! 3. **CrossSourceDedup (this module)** — collapses logically identical unlocks
//!    across DIFFERENT adapters (REQ DETECT-07).
//!
//! All three are required: layer 1 doesn't see across files, layer 2 doesn't see
//! across adapters, layer 3 catches what the others miss.
//!
//! # TTL choice (10 seconds default)
//!
//! Real-world simultaneity between adapters is sub-second. 10s is a generous safety
//! margin (RESEARCH.md "Pattern 3"). The SQLite `UNIQUE INDEX` (Plan 02) is the
//! belt-and-suspenders backstop if a duplicate slips past TTL.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// In-memory TTL cache for cross-source dedup.
/// NOT thread-safe by itself — wrap in `tokio::sync::Mutex` for shared access.
pub struct CrossSourceDedup {
    seen: HashMap<(u64, String), Instant>,
    ttl: Duration,
}

impl CrossSourceDedup {
    pub fn new(ttl: Duration) -> Self {
        Self {
            seen: HashMap::new(),
            ttl,
        }
    }

    /// Returns `true` if this event should be DROPPED as a duplicate.
    /// Side effect: sweeps expired entries before checking, then inserts the new
    /// observation if not a duplicate.
    pub fn is_duplicate(&mut self, app_id: u64, ach_api_name: &str) -> bool {
        let now = Instant::now();
        // Sweep expired. O(n) but n is bounded by per-session unlock count (small).
        let ttl = self.ttl;
        self.seen.retain(|_, ts| now.duration_since(*ts) < ttl);

        let key = (app_id, ach_api_name.to_string());
        if self.seen.contains_key(&key) {
            true
        } else {
            self.seen.insert(key, now);
            false
        }
    }

    /// Number of currently-tracked entries (for diagnostics).
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    /// Whether the dedup tracker has no observations.
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_observation_is_not_duplicate() {
        let mut d = CrossSourceDedup::new(Duration::from_secs(10));
        assert!(!d.is_duplicate(480, "ACH_X"));
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn repeat_observation_within_ttl_is_duplicate() {
        let mut d = CrossSourceDedup::new(Duration::from_secs(10));
        assert!(!d.is_duplicate(480, "ACH_X"));
        assert!(d.is_duplicate(480, "ACH_X"));
        assert!(d.is_duplicate(480, "ACH_X"));
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn expired_observation_is_no_longer_duplicate() {
        let mut d = CrossSourceDedup::new(Duration::from_millis(50));
        assert!(!d.is_duplicate(480, "ACH_X"));
        std::thread::sleep(Duration::from_millis(100));
        // After TTL the entry is swept; a fresh observation is NOT a duplicate.
        assert!(!d.is_duplicate(480, "ACH_X"));
    }

    #[test]
    fn different_keys_are_independent() {
        let mut d = CrossSourceDedup::new(Duration::from_secs(10));
        assert!(!d.is_duplicate(480, "ACH_X"));
        assert!(!d.is_duplicate(481, "ACH_X"));
        assert!(!d.is_duplicate(480, "ACH_Y"));
        assert!(d.is_duplicate(480, "ACH_X"));
        assert_eq!(d.len(), 3);
    }
}
