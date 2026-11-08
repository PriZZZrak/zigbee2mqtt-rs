// Original: .upstream/zigbee-herdsman/src/utils/wait.ts

use std::time::Duration;

/// Upstream `wait(milliseconds)`.
pub async fn wait(milliseconds: u64) {
    tokio::time::sleep(Duration::from_millis(milliseconds)).await;
}
