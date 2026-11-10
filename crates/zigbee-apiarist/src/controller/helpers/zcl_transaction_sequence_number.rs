// Original: .upstream/zigbee-herdsman/src/controller/helpers/zclTransactionSequenceNumber.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use std::sync::atomic::{AtomicI16, Ordering};

/// Upstream `class ZclTransactionSequenceNumber.sequence = -1`.
static SEQUENCE: AtomicI16 = AtomicI16::new(-1);

/// Upstream `get current()`.
pub fn current() -> i16 {
    SEQUENCE.load(Ordering::SeqCst)
}

/// Upstream `next()` — `(sequence + 1) % 256`.
pub fn next() -> u8 {
    let previous = SEQUENCE
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |sequence| {
            Some((sequence + 1) % 256)
        })
        .unwrap_or_else(|sequence| sequence);
    ((previous + 1) % 256) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_like_upstream() {
        // The global counter is shared across tests; only relative values are
        // asserted to avoid ordering surprises.
        let first = next();
        let second = next();
        assert_eq!(second, first.wrapping_add(1));
        assert_eq!(current(), i16::from(second));
    }
}
