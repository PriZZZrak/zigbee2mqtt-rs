// Original: .upstream/zigbee-herdsman/src/controller/helpers/requestQueue.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use serde_json::Value;
use tokio::sync::oneshot;

use super::request::Request;
use crate::controller::tstype::SendPolicy;
use crate::utils::datetime::now_ms;
use crate::utils::logger::logger;

const NS: &str = "zh:controller:requestqueue";

/// Upstream `RequestQueue extends Set<Request>` — requestQueue.ts:12.
#[derive(Default)]
pub struct RequestQueue {
    pub device_ieee_address: String,
    pub endpoint_id: u8,
    requests: Vec<Request>,
    send_in_progress: bool,
}

impl RequestQueue {
    /// Upstream `constructor(endpoint)`.
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

    #[cfg(test)]
    fn push_request(&mut self, request: Request) {
        self.requests.push(request);
    }

    /// Upstream `queue(request)` — requestQueue.ts:67.
    pub async fn queue(&mut self, mut request: Request) -> Result<Value, String> {
        logger().debug(
            &format!(
                "Request Queue ({}/{}): Sending when active. Expires: {}",
                self.device_ieee_address, self.endpoint_id, request.expires
            ),
            NS,
        );
        let (sender, receiver) = oneshot::channel();
        request.add_callback(sender);
        self.requests.push(request);
        receiver
            .await
            .unwrap_or_else(|_| Err("Request dropped".to_string()))
    }

    /// Upstream `filter(newRequest)` — requestQueue.ts:78.
    pub fn filter(&mut self, new_request: &mut Request) {
        if self.requests.is_empty() {
            return;
        }
        let cluster_id = new_request.frame.cluster.ID;
        let payload = new_request.frame.payload.clone();
        let command_id = new_request.frame.command.ID;

        logger().debug(
            &format!(
                "Request Queue ({}/{}): ZCL {} command, filter requests. Before: {}",
                self.device_ieee_address,
                self.endpoint_id,
                new_request.frame.command.name,
                self.requests.len()
            ),
            NS,
        );

        let mut index = 0usize;
        while index < self.requests.len() {
            let request = &mut self.requests[index];
            if matches!(
                request.send_policy,
                Some(SendPolicy::Bulk) | Some(SendPolicy::Queue) | Some(SendPolicy::Immediate)
            ) {
                index += 1;
                continue;
            }

            if request.frame.cluster.ID != cluster_id || request.frame.command.ID != command_id {
                index += 1;
                continue;
            }

            let remove = match new_request.send_policy {
                Some(SendPolicy::KeepPayload) if request.frame.payload == payload => {
                    logger().debug(
                        &format!(
                            "Request Queue ({}/{}): Merge duplicate request",
                            self.device_ieee_address, self.endpoint_id
                        ),
                        NS,
                    );
                    new_request.move_callbacks(request);
                    true
                }
                Some(SendPolicy::KeepCommand) | Some(SendPolicy::KeepCmdUndiv)
                    if request.frame.payload.is_array() =>
                {
                    let old_payload = request
                        .frame
                        .payload
                        .as_array()
                        .cloned()
                        .unwrap_or_default();
                    let new_attributes = payload.as_array().cloned().unwrap_or_default();
                    let filtered = old_payload
                        .iter()
                        .filter(|old| {
                            let old_attr = old.get("attrId").and_then(Value::as_u64);
                            !new_attributes
                                .iter()
                                .any(|new| new.get("attrId").and_then(Value::as_u64) == old_attr)
                        })
                        .cloned()
                        .collect::<Vec<_>>();

                    if filtered.is_empty() {
                        logger().debug(
                            &format!(
                                "Request Queue ({}/{}): Remove & reject request",
                                self.device_ieee_address, self.endpoint_id
                            ),
                            NS,
                        );
                        if request.frame.payload == payload {
                            new_request.move_callbacks(request);
                        } else {
                            request.reject(Some("Request superseded".to_string()));
                        }
                        true
                    } else {
                        if new_request.send_policy != Some(SendPolicy::KeepCmdUndiv) {
                            request.frame.payload = Value::Array(filtered);
                            logger().debug(
                                &format!(
                                    "Request Queue ({}/{}): Remove commands from request",
                                    self.device_ieee_address, self.endpoint_id
                                ),
                                NS,
                            );
                        }
                        false
                    }
                }
                _ => false,
            };

            if remove {
                self.requests.remove(index);
            } else {
                index += 1;
            }
        }
        logger().debug(
            &format!(
                "Request Queue ({}/{}): After: {}",
                self.device_ieee_address,
                self.endpoint_id,
                self.requests.len()
            ),
            NS,
        );
    }

    /// Upstream `send(fastPolling)` — requestQueue.ts:23.
    pub async fn send(&mut self, fast_polling: bool) {
        if self.requests.is_empty() {
            return;
        }
        if !fast_polling && self.send_in_progress {
            logger().debug(
                &format!(
                    "Request Queue ({}/{}): sendPendingRequests already in progress",
                    self.device_ieee_address, self.endpoint_id
                ),
                NS,
            );
            return;
        }
        self.send_in_progress = true;

        // Remove expired requests first.
        let now = now_ms();
        let mut index = 0usize;
        while index < self.requests.len() {
            if now > self.requests[index].expires {
                logger().debug(
                    &format!(
                        "Request Queue ({}/{}): discard after timeout. Size before: {}",
                        self.device_ieee_address,
                        self.endpoint_id,
                        self.requests.len()
                    ),
                    NS,
                );
                let mut request = self.requests.remove(index);
                request.reject(None);
            } else {
                index += 1;
            }
        }

        logger().debug(
            &format!(
                "Request Queue ({}/{}): send pending requests ({}, {})",
                self.device_ieee_address,
                self.endpoint_id,
                self.requests.len(),
                fast_polling
            ),
            NS,
        );

        let mut index = 0usize;
        while index < self.requests.len() {
            if fast_polling || self.requests[index].send_policy != Some(SendPolicy::Bulk) {
                let mut request = self.requests.remove(index);
                match request.send().await {
                    Ok(result) => {
                        logger().debug(
                            &format!(
                                "Request Queue ({}/{}): send success",
                                self.device_ieee_address, self.endpoint_id
                            ),
                            NS,
                        );
                        request.resolve(result);
                    }
                    Err(error) => {
                        logger().debug(
                            &format!(
                                "Request Queue ({}/{}): send failed, expires in {} seconds ({error})",
                                self.device_ieee_address,
                                self.endpoint_id,
                                (request.expires.saturating_sub(now)) as f64 / 1000.0
                            ),
                            NS,
                        );
                        self.requests.insert(index, request);
                        index += 1;
                    }
                }
            } else {
                index += 1;
            }
        }
        self.send_in_progress = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::helpers::request::{default_send_policy, BoxedRequestFuture, Request};
    use crate::zspec::zcl::zcl_frame::Frame;
    use serde_json::json;
    use tokio::sync::oneshot;

    fn ok(value: Value) -> BoxedRequestFuture {
        Box::pin(async move { Ok(value) })
    }

    fn read_frame(payload: Value) -> Frame {
        Frame::create(0, 0, true, None, 1, "read", 6, payload, json!({}), 0)
    }

    fn write_frame(payload: Value) -> Frame {
        Frame::create(0, 0, true, None, 1, "write", 6, payload, json!({}), 0)
    }

    fn request_with_callback(
        frame: Frame,
        timeout: u64,
    ) -> (Request, oneshot::Receiver<Result<Value, String>>) {
        let mut request = Request::new(move || ok(json!({"sent": true})), frame, timeout);
        let (sender, receiver) = oneshot::channel();
        request.add_callback(sender);
        (request, receiver)
    }

    #[test]
    fn default_policies_match_upstream() {
        assert_eq!(default_send_policy(0), Some(SendPolicy::KeepPayload));
        assert_eq!(default_send_policy(1), Some(SendPolicy::Immediate));
        assert_eq!(default_send_policy(2), Some(SendPolicy::KeepCommand));
        assert_eq!(default_send_policy(3), Some(SendPolicy::KeepCmdUndiv));
        assert_eq!(default_send_policy(5), Some(SendPolicy::KeepCommand));
        assert_eq!(default_send_policy(14), Some(SendPolicy::KeepPayload));
        assert_eq!(default_send_policy(22), Some(SendPolicy::Immediate));
        assert_eq!(default_send_policy(255), None);
    }

    #[tokio::test]
    async fn sends_and_resolves_successful_request() {
        let mut queue = RequestQueue::new("0x00124b0000000001", 1);
        let (request, receiver) = request_with_callback(read_frame(json!([{"attrId": 0}])), 10_000);
        queue.push_request(request);
        queue.send(false).await;
        assert_eq!(receiver.await.expect("callback"), Ok(json!({"sent": true})));
        assert!(queue.is_empty());
    }

    #[tokio::test]
    async fn expired_request_is_rejected_before_send() {
        let mut queue = RequestQueue::new("0x00124b0000000001", 1);
        let (mut request, receiver) = request_with_callback(read_frame(json!([{"attrId": 0}])), 0);
        request.expires = now_ms().saturating_sub(1);
        queue.push_request(request);
        queue.send(false).await;
        assert_eq!(
            receiver.await.expect("callback"),
            Err("Request rejected before first send".to_string())
        );
        assert!(queue.is_empty());
    }

    #[tokio::test]
    async fn bulk_requests_wait_for_fast_polling() {
        let mut queue = RequestQueue::new("0x00124b0000000001", 1);
        let (mut request, receiver) =
            request_with_callback(read_frame(json!([{"attrId": 0}])), 10_000);
        request.send_policy = Some(SendPolicy::Bulk);
        queue.push_request(request);
        queue.send(false).await;
        assert_eq!(queue.len(), 1);
        queue.send(true).await;
        assert!(receiver.await.expect("callback").is_ok());
        assert!(queue.is_empty());
    }

    #[tokio::test]
    async fn filter_merges_identical_keep_payload_requests() {
        let mut queue = RequestQueue::new("0x00124b0000000001", 1);
        let payload = json!([{"attrId": 0}]);
        let (old, old_receiver) = request_with_callback(read_frame(payload.clone()), 10_000);
        queue.push_request(old);

        let (mut new, new_receiver) = request_with_callback(read_frame(payload), 10_000);
        new.send_policy = Some(SendPolicy::KeepPayload);
        queue.filter(&mut new);
        assert!(queue.is_empty());

        queue.push_request(new);
        queue.send(false).await;
        assert!(new_receiver.await.expect("new callback").is_ok());
        assert!(old_receiver.await.expect("old callback").is_ok());
    }

    #[tokio::test]
    async fn filter_rejects_superseded_write() {
        let mut queue = RequestQueue::new("0x00124b0000000001", 1);
        // Every old attribute is also present in the new request, but the
        // payloads differ, so the old request is superseded and rejected.
        let (mut old, old_receiver) =
            request_with_callback(write_frame(json!([{"attrId": 1}])), 10_000);
        old.send_policy = Some(SendPolicy::KeepCommand);
        queue.push_request(old);

        let (mut new, _new_receiver) =
            request_with_callback(write_frame(json!([{"attrId": 1}, {"attrId": 2}])), 10_000);
        new.send_policy = Some(SendPolicy::KeepCommand);
        queue.filter(&mut new);
        assert!(queue.is_empty());
        assert_eq!(
            old_receiver.await.expect("old callback"),
            Err("Request superseded".to_string())
        );
    }

    #[tokio::test]
    async fn filter_removes_duplicate_attributes_from_old_write() {
        let mut queue = RequestQueue::new("0x00124b0000000001", 1);
        let (mut old, _old_receiver) =
            request_with_callback(write_frame(json!([{"attrId": 1}, {"attrId": 2}])), 10_000);
        old.send_policy = Some(SendPolicy::KeepCommand);
        queue.push_request(old);

        let (mut new, _new_receiver) =
            request_with_callback(write_frame(json!([{"attrId": 1}])), 10_000);
        new.send_policy = Some(SendPolicy::KeepCommand);
        queue.filter(&mut new);

        assert_eq!(queue.len(), 1);
        // The remaining request is the old one, with attrId 1 removed.
        let remaining = &queue.requests[0];
        assert_eq!(remaining.frame.payload, json!([{"attrId": 2}]));
    }
}
