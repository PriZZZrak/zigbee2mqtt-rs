// Original: .upstream/zigbee-herdsman/src/controller/helpers/request.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use std::future::Future;
use std::pin::Pin;

use serde_json::Value;
use tokio::sync::oneshot;

use crate::controller::tstype::SendPolicy;
use crate::utils::datetime::now_ms;
use crate::zspec::zcl::zcl_frame::Frame;

pub type BoxedRequestFuture = Pin<Box<dyn Future<Output = Result<Value, String>> + Send>>;

/// Upstream `Request.defaultSendPolicy` — request.ts:7.
pub fn default_send_policy(command_id: u8) -> Option<SendPolicy> {
    Some(match command_id {
        0 | 6 | 8 | 10 | 12 | 14 | 15 | 17 | 19 | 21 => SendPolicy::KeepPayload,
        1 | 4 | 7 | 9 | 11 | 13 | 16 | 18 | 20 | 22 => SendPolicy::Immediate,
        2 | 5 => SendPolicy::KeepCommand,
        3 => SendPolicy::KeepCmdUndiv,
        _ => return None,
    })
}

type Callback = oneshot::Sender<Result<Value, String>>;

/// Upstream `Request<Type>` — request.ts:30.
pub struct Request {
    pub frame: Frame,
    pub expires: u64,
    pub send_policy: Option<SendPolicy>,
    function: Box<dyn Fn() -> BoxedRequestFuture + Send>,
    callbacks: Vec<Callback>,
    last_error: String,
}

impl Request {
    /// Upstream `constructor(func, frame, timeout, sendPolicy?, lastError?)`.
    pub fn new(
        function: impl Fn() -> BoxedRequestFuture + Send + 'static,
        frame: Frame,
        timeout: u64,
    ) -> Self {
        Self::new_with_policy(function, frame, timeout, None)
    }

    /// Upstream optional `sendPolicy` constructor argument.
    pub fn new_with_policy(
        function: impl Fn() -> BoxedRequestFuture + Send + 'static,
        frame: Frame,
        timeout: u64,
        send_policy: Option<SendPolicy>,
    ) -> Self {
        let send_policy = send_policy.or_else(|| default_send_policy(frame.command.ID));
        Self {
            frame,
            expires: timeout + now_ms(),
            send_policy,
            function: Box::new(function),
            callbacks: Vec::new(),
            last_error: "Request rejected before first send".to_string(),
        }
    }

    /// Upstream `moveCallbacks(from)` — request.ts:52.
    pub fn move_callbacks(&mut self, from: &mut Request) {
        self.callbacks.append(&mut from.callbacks);
        from.callbacks.clear();
    }

    /// Upstream `addCallbacks(resolve, reject)` — request.ts:58.
    pub fn add_callback(&mut self, callback: Callback) {
        self.callbacks.push(callback);
    }

    /// Upstream `reject(error?)` — request.ts:63.
    pub fn reject(&mut self, error: Option<String>) {
        let error = error.unwrap_or_else(|| self.last_error.clone());
        for callback in self.callbacks.drain(..) {
            let _ = callback.send(Err(error.clone()));
        }
    }

    /// Upstream `resolve(value)` — request.ts:70.
    pub fn resolve(&mut self, value: Value) {
        for callback in self.callbacks.drain(..) {
            let _ = callback.send(Ok(value.clone()));
        }
    }

    /// Upstream `send()` — request.ts:77.
    pub async fn send(&mut self) -> Result<Value, String> {
        match (self.function)().await {
            Ok(value) => Ok(value),
            Err(error) => {
                self.last_error = error.clone();
                Err(error)
            }
        }
    }

    pub fn last_error(&self) -> &str {
        &self.last_error
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zspec::zcl::zcl_frame::Frame;
    use serde_json::json;

    fn frame() -> Frame {
        Frame::create(0, 0, true, None, 1, "read", 6, json!([]), json!({}), 0)
    }

    #[test]
    fn explicit_send_policy_overrides_default() {
        let request = Request::new_with_policy(
            || Box::pin(async { Ok(serde_json::Value::Null) }),
            frame(),
            1000,
            Some(SendPolicy::Bulk),
        );
        assert_eq!(request.send_policy, Some(SendPolicy::Bulk));

        let request = Request::new_with_policy(
            || Box::pin(async { Ok(serde_json::Value::Null) }),
            frame(),
            1000,
            None,
        );
        assert_eq!(request.send_policy, Some(SendPolicy::KeepPayload));
    }
}
