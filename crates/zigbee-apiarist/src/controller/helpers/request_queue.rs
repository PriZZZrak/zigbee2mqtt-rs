// Original: .upstream/zigbee-herdsman/src/controller/helpers/requestQueue.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use serde_json::Value;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::oneshot;

use super::request::Request;
use crate::controller::tstype::SendPolicy;
use crate::utils::datetime::now_ms;
use crate::utils::logger::logger;

const NS: &str = "zh:controller:requestqueue";

/// Upstream `RequestQueue extends Set<Request>` — requestQueue.ts:12.
pub struct RequestQueue<T = Value> {
    pub device_ieee_address: String,
    pub endpoint_id: u8,
    requests: Vec<Request<T>>,
    send_in_progress: bool,
    /// Upstream `get size()` — observable without acquiring the queue lock.
    size: Arc<AtomicUsize>,
}

impl<T: Clone + Send + 'static> Default for RequestQueue<T> {
    fn default() -> Self {
        Self {
            device_ieee_address: String::new(),
            endpoint_id: 0,
            requests: Vec::new(),
            send_in_progress: false,
            size: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl<T: Clone + Send + 'static> RequestQueue<T> {
    /// Upstream `constructor(endpoint)`.
    pub fn new(device_ieee_address: impl Into<String>, endpoint_id: u8) -> Self {
        Self {
            device_ieee_address: device_ieee_address.into(),
            endpoint_id,
            requests: Vec::new(),
            send_in_progress: false,
            size: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn len(&self) -> usize {
        self.requests.len()
    }

    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    /// Upstream `get size()`.
    pub fn size(&self) -> usize {
        self.size.load(Ordering::SeqCst)
    }

    /// Handle for `hasPendingRequests()` without locking the queue.
    pub fn size_handle(&self) -> Arc<AtomicUsize> {
        self.size.clone()
    }

    fn sync_size(&self) {
        self.size.store(self.requests.len(), Ordering::SeqCst);
    }

    #[cfg(test)]
    fn push_request(&mut self, request: Request<T>) {
        self.requests.push(request);
        self.sync_size();
    }

    /// Upstream `queue(request)` — requestQueue.ts:67.
    ///
    /// Split in the Rust port: `enqueue` performs the upstream insertion and
    /// returns the response channel, `queue` awaits it. Keeping the await out of
    /// `enqueue` lets callers release the queue lock before waiting (upstream
    /// queues a promise without any lock).
    pub fn enqueue(&mut self, mut request: Request<T>) -> oneshot::Receiver<Result<T, String>> {
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
        self.sync_size();
        receiver
    }

    /// Upstream `queue(request)` — requestQueue.ts:67.
    pub async fn queue(&mut self, request: Request<T>) -> Result<T, String> {
        self.enqueue(request)
            .await
            .unwrap_or_else(|_| Err("Request dropped".to_string()))
    }

    /// Upstream `filter(newRequest)` — requestQueue.ts:78.
    pub fn filter(&mut self, new_request: &mut Request<T>) {
        if self.requests.is_empty() {
            return;
        }
        let cluster_id = new_request.frame.cluster.ID as u16;
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

            if request.frame.cluster.ID as u16 != cluster_id
                || request.frame.command.ID != command_id
            {
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
        self.sync_size();
    }

    /// Upstream `send(fastPolling)` — requestQueue.ts:23.
    ///
    /// Split so callers can release the queue lock while requests are in
    /// flight (upstream does not hold a lock at all). Returns `None` when the
    /// call was skipped (empty queue or a send already in progress).
    pub fn take_for_send(
        &mut self,
        fast_polling: bool,
    ) -> Option<(Vec<Request<T>>, Vec<Request<T>>)> {
        if self.requests.is_empty() {
            return None;
        }
        if !fast_polling && self.send_in_progress {
            logger().debug(
                &format!(
                    "Request Queue ({}/{}): sendPendingRequests already in progress",
                    self.device_ieee_address, self.endpoint_id
                ),
                NS,
            );
            return None;
        }
        self.send_in_progress = true;

        let now = now_ms();
        let mut expired = Vec::new();
        let mut pending = Vec::new();
        for request in std::mem::take(&mut self.requests) {
            if now > request.expires {
                logger().debug(
                    &format!(
                        "Request Queue ({}/{}): discard after timeout. Size before: {}",
                        self.device_ieee_address,
                        self.endpoint_id,
                        pending.len() + expired.len() + 1
                    ),
                    NS,
                );
                expired.push(request);
            } else {
                pending.push(request);
            }
        }
        self.sync_size();

        logger().debug(
            &format!(
                "Request Queue ({}/{}): send pending requests ({}, {})",
                self.device_ieee_address,
                self.endpoint_id,
                pending.len(),
                fast_polling
            ),
            NS,
        );

        let mut to_send = Vec::new();
        let mut remaining = Vec::new();
        for request in pending {
            if fast_polling || request.send_policy != Some(SendPolicy::Bulk) {
                to_send.push(request);
            } else {
                remaining.push(request);
            }
        }
        self.requests = remaining;
        self.sync_size();
        Some((to_send, expired))
    }

    /// Puts a request back after a failed send attempt.
    pub fn requeue(&mut self, request: Request<T>) {
        self.requests.push(request);
        self.sync_size();
    }

    /// Clears the in-progress marker once a send round finished.
    pub fn finish_send(&mut self) {
        self.send_in_progress = false;
        self.sync_size();
    }

    /// Upstream `send(fastPolling)` — requestQueue.ts:23.
    pub async fn send(&mut self, fast_polling: bool) {
        let Some((to_send, mut expired)) = self.take_for_send(fast_polling) else {
            return;
        };
        for request in &mut expired {
            request.reject(None);
        }
        let now = now_ms();
        for mut request in to_send {
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
                    self.requeue(request);
                }
            }
        }
        self.finish_send();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::helpers::request::{default_send_policy, BoxedRequestFuture, Request};
    use crate::zspec::zcl::zcl_frame::Frame;
    use serde_json::json;
    use tokio::sync::oneshot;

    fn ok(value: Value) -> BoxedRequestFuture<Value> {
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
    ) -> (Request<Value>, oneshot::Receiver<Result<Value, String>>) {
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
    async fn size_tracks_queued_requests_without_locking() {
        let mut queue = RequestQueue::new("0x00124b0000000001", 1);
        let size_handle = queue.size_handle();
        assert_eq!(size_handle.load(Ordering::SeqCst), 0);

        let (request, receiver) = request_with_callback(read_frame(json!([{"attrId": 0}])), 10_000);
        queue.push_request(request);
        assert_eq!(size_handle.load(Ordering::SeqCst), 1);
        assert_eq!(queue.size(), 1);

        queue.send(false).await;
        assert!(receiver.await.expect("callback").is_ok());
        assert_eq!(size_handle.load(Ordering::SeqCst), 0);
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
