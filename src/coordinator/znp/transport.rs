/// Async ZNP serial transport.
///
/// Provides a split channel interface:
///   - `ZnpSender`  – send SREQ/AREQ frames
///   - `ZnpReceiver` – receive incoming SRSP/AREQ frames
///
/// SREQ/SRSP pairing is handled internally: `request()` sends an SREQ and
/// waits for the matching SRSP (same cmd0 subsystem bits + cmd1).
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures::SinkExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, oneshot};
use tokio_serial::SerialPortBuilderExt;
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::{error, trace, warn};

use super::frame::{FrameType, Subsystem, ZnpCodec, ZnpFrame};
use crate::error::{Error, Result};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
// `event_tx` (AREQ events to the bridge) uses try_send and drops on overflow
// (see dispatch()) rather than blocking the serial read loop -- sized well
// above typical steady-state traffic so a burst (e.g. many devices reporting
// right after coordinator restart) doesn't silently lose messages.
const CHANNEL_CAPACITY: usize = 256;

// ── Public events that consumers receive ─────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ZnpEvent {
    ResetInd,
    EndDeviceAnnceInd(Vec<u8>),
    LeaveInd(Vec<u8>),
    AfIncomingMsg(Vec<u8>),
    AfDataConfirm(Vec<u8>),
    ActiveEpRsp(Vec<u8>),
    SimpleDescRsp(Vec<u8>),
    NodeDescRsp(Vec<u8>),
    IeeeAddrRsp(Vec<u8>),
    BindRsp(Vec<u8>),
    TcDevInd(Vec<u8>),
    StateChangeInd,
    Other,
}

// Internal message types for the actor task
enum ActorMsg {
    Send(ZnpFrame),
    Request {
        frame: ZnpFrame,
        id: u64,
        reply_tx: oneshot::Sender<Result<ZnpFrame>>,
    },
    /// Sent by the client when a request's timeout elapses, so the actor can
    /// drop a stale `pending` entry instead of risking it being matched
    /// against a later request of the same (subsystem, cmd1) shape -- the
    /// ZNP wire format carries no request ID we could match on directly.
    Cancel(u64),
}

// ── Actor task ────────────────────────────────────────────────────────────────

struct TransportActor<S> {
    framed: Framed<S, ZnpCodec>,
    actor_rx: mpsc::Receiver<ActorMsg>,
    event_tx: mpsc::Sender<ZnpEvent>,
    pending: Option<PendingRequest>,
    /// SREQ frames that arrived while another request was in flight. ZNP has
    /// no wire-level request id, so serializing requests is the only correct
    /// way to keep a response from being delivered to the wrong caller.
    queued: VecDeque<QueuedRequest>,
}

struct QueuedRequest {
    frame: ZnpFrame,
    id: u64,
    reply_tx: oneshot::Sender<Result<ZnpFrame>>,
}

struct PendingRequest {
    id: u64,
    expected_sub: u8,
    expected_cmd1: u8,
    reply_tx: oneshot::Sender<Result<ZnpFrame>>,
}

impl<S> TransportActor<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    async fn run(mut self) {
        loop {
            tokio::select! {
                // Outgoing messages from callers
                msg = self.actor_rx.recv() => {
                    match msg {
                        None => break,
                        Some(ActorMsg::Send(frame)) => {
                            trace!(?frame, "→ ZNP send AREQ");
                            if let Err(e) = self.framed.send(frame).await {
                                error!("ZNP serial write error: {e}");
                            }
                        }
                        Some(ActorMsg::Request { frame, id, reply_tx }) => {
                            self.queued.push_back(QueuedRequest { frame, id, reply_tx });
                            self.advance().await;
                        }
                        Some(ActorMsg::Cancel(id)) => {
                            self.queued.retain(|request| request.id != id);
                            if self.pending.as_ref().is_some_and(|p| p.id == id) {
                                self.pending = None;
                                self.advance().await;
                            }
                        }
                    }
                }

                // Incoming frames from serial port
                frame = self.framed.next() => {
                    match frame {
                        None => {
                            error!("ZNP serial port closed");
                            break;
                        }
                        Some(Err(e)) => {
                            error!("ZNP serial read error: {e}");
                        }
                        Some(Ok(f)) => {
                            let completed = self.dispatch(f);
                            if completed {
                                self.advance().await;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Send the next queued SREQ when no request is in flight.
    async fn advance(&mut self) {
        while self.pending.is_none() {
            let Some(next) = self.queued.pop_front() else {
                break;
            };
            let expected_sub = next.frame.cmd0() & 0x1F;
            let expected_cmd1 = next.frame.cmd1;
            trace!(?next.frame, "→ ZNP send SREQ");
            if let Err(error) = self.framed.send(next.frame).await {
                let _ = next.reply_tx.send(Err(Error::Io(error)));
                continue;
            }
            self.pending = Some(PendingRequest {
                id: next.id,
                expected_sub,
                expected_cmd1,
                reply_tx: next.reply_tx,
            });
        }
    }

    /// Deliver a matching SRSP. Returns whether the in-flight request completed.
    fn dispatch(&mut self, frame: ZnpFrame) -> bool {
        trace!(?frame, "← ZNP recv");

        if frame.frame_type == FrameType::SRsp {
            if let Some(pending) = &self.pending {
                if frame.subsystem as u8 == pending.expected_sub
                    && frame.cmd1 == pending.expected_cmd1
                {
                    let pending = self.pending.take().unwrap();
                    let _ = pending.reply_tx.send(Ok(frame));
                    return true;
                }
                // Mismatched subsystem but there's a pending request — deliver it
                // anyway. Z-Stack 1.2 may return subsystem 0 for errors.
                if frame.cmd1 == pending.expected_cmd1 {
                    warn!(
                        "SRSP subsystem mismatch: expected 0x{:02X} got 0x{:02X}, delivering anyway",
                        pending.expected_sub, frame.subsystem as u8
                    );
                    let pending = self.pending.take().unwrap();
                    let _ = pending.reply_tx.send(Ok(frame));
                    return true;
                }
            }
            warn!(
                "Unexpected SRSP sub=0x{:02X} cmd1=0x{:02X}",
                frame.subsystem as u8, frame.cmd1
            );
            return false;
        }

        // AREQ – convert to typed event and fan-out
        let event = match (frame.subsystem, frame.cmd1) {
            (Subsystem::Sys, 0x80) => ZnpEvent::ResetInd,
            (Subsystem::Zdo, 0x81) => ZnpEvent::IeeeAddrRsp(frame.data),
            (Subsystem::Zdo, 0x82) => ZnpEvent::NodeDescRsp(frame.data),
            (Subsystem::Zdo, 0x84) => ZnpEvent::SimpleDescRsp(frame.data),
            (Subsystem::Zdo, 0x85) => ZnpEvent::ActiveEpRsp(frame.data),
            (Subsystem::Zdo, 0xA1) => ZnpEvent::BindRsp(frame.data),
            (Subsystem::Zdo, 0xC0) => ZnpEvent::StateChangeInd,
            (Subsystem::Zdo, 0xC1) => ZnpEvent::EndDeviceAnnceInd(frame.data),
            (Subsystem::Zdo, 0xC9) => ZnpEvent::LeaveInd(frame.data),
            (Subsystem::Zdo, 0xCA) => ZnpEvent::TcDevInd(frame.data),
            (Subsystem::Af, 0x80) => ZnpEvent::AfDataConfirm(frame.data),
            (Subsystem::Af, 0x81) => ZnpEvent::AfIncomingMsg(frame.data),
            _ => ZnpEvent::Other,
        };

        if let Err(e) = self.event_tx.try_send(event) {
            warn!("ZNP event queue full, dropping: {e}");
        }
        false
    }
}

// ── Public handle ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ZnpTransport {
    actor_tx: mpsc::Sender<ActorMsg>,
    next_id: Arc<AtomicU64>,
}

impl ZnpTransport {
    /// Open the serial port and spawn the actor task.
    pub fn open(port: &str, baud: u32) -> Result<(Self, mpsc::Receiver<ZnpEvent>)> {
        let serial = tokio_serial::new(port, baud)
            .timeout(Duration::from_millis(100))
            .open_native_async()?;

        let framed = Framed::new(serial, ZnpCodec);
        let (actor_tx, actor_rx) = mpsc::channel::<ActorMsg>(CHANNEL_CAPACITY);
        let (event_tx, event_rx) = mpsc::channel::<ZnpEvent>(CHANNEL_CAPACITY);

        let actor = TransportActor {
            framed,
            actor_rx,
            event_tx,
            pending: None,
            queued: VecDeque::new(),
        };
        tokio::spawn(actor.run());

        Ok((
            Self {
                actor_tx,
                next_id: Arc::new(AtomicU64::new(0)),
            },
            event_rx,
        ))
    }

    /// Fire-and-forget AREQ send.
    pub async fn send(&self, frame: ZnpFrame) -> Result<()> {
        self.actor_tx
            .send(ActorMsg::Send(frame))
            .await
            .map_err(|_| Error::ChannelClosed)
    }

    /// Send SREQ and wait for matching SRSP.
    pub async fn request(&self, frame: ZnpFrame) -> Result<ZnpFrame> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (reply_tx, reply_rx) = oneshot::channel();
        self.actor_tx
            .send(ActorMsg::Request {
                frame,
                id,
                reply_tx,
            })
            .await
            .map_err(|_| Error::ChannelClosed)?;

        match tokio::time::timeout(REQUEST_TIMEOUT, reply_rx).await {
            Ok(result) => result.map_err(|_| Error::ChannelClosed)?,
            Err(_) => {
                // Tell the actor to drop `pending` if it's still ours, so a
                // late SRSP that finally arrives isn't misdelivered to
                // whichever request comes next with the same (subsystem, cmd1).
                let _ = self.actor_tx.send(ActorMsg::Cancel(id)).await;
                Err(Error::Timeout)
            }
        }
    }
}

/// Test-only actor over an arbitrary duplex stream, used by coordinator-level
/// fake-serial E2E tests.
#[cfg(test)]
pub(crate) fn spawn_test_transport<S>(
    framed: Framed<S, ZnpCodec>,
) -> (ZnpTransport, mpsc::Receiver<ZnpEvent>)
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (actor_tx, actor_rx) = mpsc::channel(CHANNEL_CAPACITY);
    let (event_tx, event_rx) = mpsc::channel(CHANNEL_CAPACITY);
    let actor = TransportActor {
        framed,
        actor_rx,
        event_tx,
        pending: None,
        queued: VecDeque::new(),
    };
    tokio::spawn(actor.run());
    (
        ZnpTransport {
            actor_tx,
            next_id: Arc::new(AtomicU64::new(0)),
        },
        event_rx,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::DuplexStream;

    fn sreq(cmd1: u8) -> ZnpFrame {
        ZnpFrame::sreq(Subsystem::Sys, cmd1, vec![cmd1])
    }

    fn srsp(cmd1: u8, data: u8) -> ZnpFrame {
        ZnpFrame::new(FrameType::SRsp, Subsystem::Sys, cmd1, vec![data])
    }

    fn spawn_actor() -> (
        ZnpTransport,
        Framed<DuplexStream, ZnpCodec>,
        mpsc::Receiver<ZnpEvent>,
    ) {
        let (wire, serial) = tokio::io::duplex(4096);
        let (actor_tx, actor_rx) = mpsc::channel(CHANNEL_CAPACITY);
        let (event_tx, event_rx) = mpsc::channel(CHANNEL_CAPACITY);
        let actor = TransportActor {
            framed: Framed::new(serial, ZnpCodec),
            actor_rx,
            event_tx,
            pending: None,
            queued: VecDeque::new(),
        };
        tokio::spawn(actor.run());
        (
            ZnpTransport {
                actor_tx,
                next_id: Arc::new(AtomicU64::new(0)),
            },
            Framed::new(wire, ZnpCodec),
            event_rx,
        )
    }

    #[tokio::test]
    async fn sreq_requests_are_serialized_and_lose_no_response() {
        let (transport, mut wire, _events) = spawn_actor();

        let first = tokio::spawn({
            let transport = transport.clone();
            async move { transport.request(sreq(0x01)).await }
        });
        let sent_first = wire.next().await.unwrap().unwrap();
        assert_eq!(sent_first.cmd1, 0x01);

        let second = tokio::spawn({
            let transport = transport.clone();
            async move { transport.request(sreq(0x02)).await }
        });
        // The second request must wait for the first SRSP instead of replacing
        // the pending request (the bug this queue fixes).
        assert!(
            tokio::time::timeout(Duration::from_millis(50), wire.next())
                .await
                .is_err(),
            "second SREQ was sent while the first was still outstanding"
        );

        wire.send(srsp(0x01, 0xA1)).await.unwrap();
        let first_reply = first.await.unwrap().unwrap();
        assert_eq!(first_reply.data, vec![0xA1]);

        let sent_second = tokio::time::timeout(Duration::from_millis(250), wire.next())
            .await
            .expect("queued SREQ must be sent after the first completes")
            .unwrap()
            .unwrap();
        assert_eq!(sent_second.cmd1, 0x02);

        wire.send(srsp(0x02, 0xB2)).await.unwrap();
        let second_reply = second.await.unwrap().unwrap();
        assert_eq!(second_reply.data, vec![0xB2]);
    }

    #[tokio::test]
    async fn cancelled_queued_request_is_dropped_and_does_not_block_others() {
        let (transport, mut wire, _events) = spawn_actor();
        let (first_tx, first_rx) = oneshot::channel();
        transport
            .actor_tx
            .send(ActorMsg::Request {
                frame: sreq(0x03),
                id: 100,
                reply_tx: first_tx,
            })
            .await
            .unwrap();
        assert_eq!(wire.next().await.unwrap().unwrap().cmd1, 0x03);

        let (second_tx, second_rx) = oneshot::channel();
        transport
            .actor_tx
            .send(ActorMsg::Request {
                frame: sreq(0x04),
                id: 101,
                reply_tx: second_tx,
            })
            .await
            .unwrap();
        transport
            .actor_tx
            .send(ActorMsg::Cancel(101))
            .await
            .unwrap();
        assert!(
            second_rx.await.is_err(),
            "cancelled caller must observe a closed reply channel"
        );

        wire.send(srsp(0x03, 0xC3)).await.unwrap();
        assert_eq!(first_rx.await.unwrap().unwrap().data, vec![0xC3]);
        assert!(
            tokio::time::timeout(Duration::from_millis(50), wire.next())
                .await
                .is_err(),
            "cancelled SREQ must never reach the wire"
        );
    }

    #[tokio::test]
    async fn mismatched_subsystem_response_is_still_delivered() {
        let (transport, mut wire, _events) = spawn_actor();
        let request = tokio::spawn({
            let transport = transport.clone();
            async move { transport.request(sreq(0x05)).await }
        });
        assert_eq!(wire.next().await.unwrap().unwrap().cmd1, 0x05);
        // Z-Stack 1.2 error responses may use subsystem 0.
        wire.send(ZnpFrame::new(
            FrameType::SRsp,
            Subsystem::Zdo,
            0x05,
            vec![0xEE],
        ))
        .await
        .unwrap();
        assert_eq!(request.await.unwrap().unwrap().data, vec![0xEE]);
    }
}
