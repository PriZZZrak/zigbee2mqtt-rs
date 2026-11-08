// Original: .upstream/zigbee-herdsman/src/utils/waitress.ts

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::{oneshot, Mutex};

type Validator<P, M> = Arc<dyn Fn(&P, &M) -> bool + Send + Sync>;
type TimeoutFormatter<M> = Arc<dyn Fn(&M, u64) -> String + Send + Sync>;

struct Waiter<P> {
    matcher: (),
    sender: Option<oneshot::Sender<Result<P, String>>>,
    timedout: bool,
    _marker: std::marker::PhantomData<P>,
}

struct Inner<P, M> {
    waiters: HashMap<u64, (M, Option<oneshot::Sender<Result<P, String>>>)>,
    timedout: HashMap<u64, bool>,
}

/// Upstream `Waitress<TPayload, TMatcher>`.
pub struct Waitress<P, M> {
    inner: Arc<Mutex<Inner<P, M>>>,
    validator: Validator<P, M>,
    timeout_formatter: TimeoutFormatter<M>,
    current_id: AtomicU64,
}

impl<P, M> Waitress<P, M>
where
    P: Clone + Send + 'static,
    M: Clone + Send + 'static,
{
    pub fn new(
        validator: impl Fn(&P, &M) -> bool + Send + Sync + 'static,
        timeout_formatter: impl Fn(&M, u64) -> String + Send + Sync + 'static,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                waiters: HashMap::new(),
                timedout: HashMap::new(),
            })),
            validator: Arc::new(validator),
            timeout_formatter: Arc::new(timeout_formatter),
            current_id: AtomicU64::new(0),
        }
    }

    /// Upstream `waitFor(matcher, timeout)` returning `(ID, receiver)`.
    pub async fn wait_for(
        &self,
        matcher: M,
        timeout: u64,
    ) -> (u64, oneshot::Receiver<Result<P, String>>) {
        let id = self.current_id.fetch_add(1, Ordering::SeqCst) + 1;
        let (sender, receiver) = oneshot::channel();
        self.inner
            .lock()
            .await
            .waiters
            .insert(id, (matcher.clone(), Some(sender)));

        let inner = self.inner.clone();
        let formatter = self.timeout_formatter.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(timeout)).await;
            let mut state = inner.lock().await;
            if let Some((_, Some(sender))) = state.waiters.remove(&id) {
                let _ = sender.send(Err(formatter(&matcher, timeout)));
            }
            state.timedout.insert(id, true);
        });

        (id, receiver)
    }

    /// Upstream `resolve(payload)`.
    pub async fn resolve(&self, payload: &P) -> bool {
        let mut state = self.inner.lock().await;
        let ids: Vec<u64> = state
            .waiters
            .iter()
            .filter(|(id, (matcher, _))| {
                !state.timedout.get(id).copied().unwrap_or(false)
                    && (self.validator)(payload, matcher)
            })
            .map(|(id, _)| *id)
            .collect();
        for id in &ids {
            if let Some((_, Some(sender))) = state.waiters.remove(id) {
                let _ = sender.send(Ok(payload.clone()));
            }
        }
        !ids.is_empty()
    }

    /// Upstream `reject(payload, message)`.
    pub async fn reject(&self, payload: &P, message: &str) -> bool {
        let mut state = self.inner.lock().await;
        let ids: Vec<u64> = state
            .waiters
            .iter()
            .filter(|(id, (matcher, _))| {
                !state.timedout.get(id).copied().unwrap_or(false)
                    && (self.validator)(payload, matcher)
            })
            .map(|(id, _)| *id)
            .collect();
        for id in &ids {
            if let Some((_, Some(sender))) = state.waiters.remove(id) {
                let _ = sender.send(Err(message.to_string()));
            }
        }
        !ids.is_empty()
    }

    pub async fn remove(&self, id: u64) {
        self.inner.lock().await.waiters.remove(&id);
    }

    pub async fn clear(&self) {
        self.inner.lock().await.waiters.clear();
    }

    pub async fn size(&self) -> usize {
        self.inner.lock().await.waiters.len()
    }
}
