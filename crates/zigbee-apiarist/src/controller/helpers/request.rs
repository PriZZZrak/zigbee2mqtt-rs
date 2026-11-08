// Original: .upstream/zigbee-herdsman/src/controller/helpers/request.ts

use std::future::Future;
use std::pin::Pin;

use serde_json::Value;

use crate::zspec::zcl::zcl_frame::Frame;

pub type BoxedRequestFuture = Pin<Box<dyn Future<Output = Result<Value, String>> + Send>>;

/// Upstream `Request.defaultSendPolicy`.
pub fn default_send_policy(command_id: u8) -> Option<&'static str> {
    Some(match command_id {
        0 | 6 | 8 | 10 | 12 | 14 | 15 | 17 | 19 | 21 => "keep-payload",
        1 | 4 | 7 | 9 | 11 | 13 | 16 | 18 | 20 | 22 => "immediate",
        2 | 5 => "keep-command",
        3 => "keep-cmd-undiv",
        _ => return None,
    })
}

/// Upstream `Request<Type>`.
pub struct Request {
    pub frame: Frame,
    pub expires: u64,
    pub send_policy: Option<String>,
    last_error: Option<String>,
    function: Box<dyn Fn() -> BoxedRequestFuture + Send>,
}

impl Request {
    pub fn new(function: impl Fn() -> BoxedRequestFuture + Send + 'static, frame: Frame) -> Self {
        let send_policy = default_send_policy(frame.command.ID).map(str::to_string);
        Self {
            frame,
            expires: now_ms() + 10_000,
            send_policy,
            last_error: None,
            function: Box::new(function),
        }
    }

    pub async fn send(&mut self) -> Result<Value, String> {
        match (self.function)().await {
            Ok(value) => Ok(value),
            Err(error) => {
                self.last_error = Some(error.clone());
                Err(error)
            }
        }
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}
