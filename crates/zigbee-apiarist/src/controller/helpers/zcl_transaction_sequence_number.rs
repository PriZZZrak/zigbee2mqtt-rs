// Original: .upstream/zigbee-herdsman/src/controller/helpers/zclTransactionSequenceNumber.ts

use std::sync::atomic::{AtomicU8, Ordering};

static SEQUENCE: AtomicU8 = AtomicU8::new(255);

/// Upstream `zclTransactionSequenceNumber.next()`: wraps 255 -> 0.
pub fn next() -> u8 {
    SEQUENCE
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |sequence| {
            Some(sequence.wrapping_add(1))
        })
        .unwrap_or_else(|sequence| sequence)
        .wrapping_add(1)
}

pub fn current() -> u8 {
    SEQUENCE.load(Ordering::SeqCst)
}
