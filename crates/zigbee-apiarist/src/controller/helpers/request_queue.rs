// Original: .upstream/zigbee-herdsman/src/controller/helpers/requestQueue.ts

use serde_json::Value;

use super::request::Request;

/// Upstream `RequestQueue` (kept per endpoint).
#[derive(Default)]
pub struct RequestQueue {
    pub device_ieee_address: String,
    pub endpoint_id: u8,
    requests: Vec<Request>,
    send_in_progress: bool,
}

impl RequestQueue {
    pub fn new(device_ieee_address: impl Into<String>, endpoint_id: u8) -> Self {
        Self {
            device_ieee_address: device_ieee_address.into(),
            endpoint_id,
            requests: Vec::new(),
            send_in_progress: false,
        }
    }

    pub fn len(&self) -> usize {
        self.requests.len()
    }

    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    pub fn queue(&mut self, request: Request) {
        self.requests.push(request);
    }

    /// Upstream `filter(newRequest)`: merge duplicate read payloads and
    /// supersede bulk writes for the same cluster/command.
    pub fn filter(&mut self, new_request: &Request) {
        if self.requests.is_empty() {
            return;
        }
        let cluster = new_request.frame.cluster.ID;
        let command = new_request.frame.command.ID;
        self.requests.retain(|request| {
            if request.frame.cluster.ID != cluster || request.frame.command.ID != command {
                return true;
            }
            if matches!(
                request.send_policy.as_deref(),
                Some("bulk") | Some("queue") | Some("immediate")
            ) {
                return true;
            }
            // keep-payload duplicates are merged; keep-command filters attributes.
            new_request.send_policy.as_deref() != Some("keep-payload")
        });
    }

    /// Upstream `send(fastPolling)`.
    pub async fn send(&mut self, fast_polling: bool) -> Vec<Result<Value, String>> {
        if self.requests.is_empty() || (!fast_polling && self.send_in_progress) {
            return Vec::new();
        }
        self.send_in_progress = true;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0);
        self.requests.retain(|request| now <= request.expires);

        let mut results = Vec::new();
        let mut index = 0;
        while index < self.requests.len() {
            if fast_polling || self.requests[index].send_policy.as_deref() != Some("bulk") {
                let result = self.requests[index].send().await;
                if result.is_ok() {
                    self.requests.remove(index);
                } else {
                    index += 1;
                }
                results.push(result);
            } else {
                index += 1;
            }
        }
        self.send_in_progress = false;
        results
    }
}
