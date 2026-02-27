use sha2::{Sha256, Digest};

/// OPI-compatible cumulative event hash computation.
///
/// Each block's event hash is computed as:
///   SHA256(previous_hash + SHA256(event_string))
///
/// This produces a running hash over all events across blocks.
pub struct EventHasher {
    current_hash: [u8; 32],
}

impl EventHasher {
    /// Create a new hasher with the initial (zero) hash.
    pub fn new() -> Self {
        Self {
            current_hash: [0u8; 32],
        }
    }

    /// Create a hasher continuing from a previous cumulative hash.
    pub fn from_previous(hash: [u8; 32]) -> Self {
        Self {
            current_hash: hash,
        }
    }

    /// Feed a single event string into the cumulative hash.
    pub fn update(&mut self, event: &str) {
        let event_hash = Sha256::digest(event.as_bytes());
        let mut hasher = Sha256::new();
        hasher.update(&self.current_hash);
        hasher.update(&event_hash);
        self.current_hash = hasher.finalize().into();
    }

    /// Get the current cumulative hash.
    pub fn finalize(&self) -> [u8; 32] {
        self.current_hash
    }

    /// Get the current hash as a hex string.
    pub fn finalize_hex(&self) -> String {
        hex::encode(self.current_hash)
    }
}

impl Default for EventHasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_hasher_deterministic() {
        let mut h1 = EventHasher::new();
        h1.update("test_event");
        let mut h2 = EventHasher::new();
        h2.update("test_event");
        assert_eq!(h1.finalize(), h2.finalize());
    }

    #[test]
    fn test_event_hasher_cumulative() {
        let mut h1 = EventHasher::new();
        h1.update("event1");
        h1.update("event2");

        let mut h2 = EventHasher::new();
        h2.update("event1");
        // Different second event should produce different hash
        h2.update("event3");
        assert_ne!(h1.finalize(), h2.finalize());
    }

    #[test]
    fn test_event_hash_empty() {
        let h = EventHasher::new();
        assert_eq!(h.finalize(), [0u8; 32]);
    }

    #[test]
    fn test_event_hash_from_previous() {
        let mut h1 = EventHasher::new();
        h1.update("event1");
        let hash1 = h1.finalize();

        let mut h2 = EventHasher::from_previous(hash1);
        h2.update("event2");

        let mut h3 = EventHasher::new();
        h3.update("event1");
        h3.update("event2");

        assert_eq!(h2.finalize(), h3.finalize());
    }

    #[test]
    fn test_event_hash_hex() {
        let h = EventHasher::new();
        let hex = h.finalize_hex();
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
