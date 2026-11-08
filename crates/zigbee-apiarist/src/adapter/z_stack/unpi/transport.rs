// Original: .upstream/zigbee-herdsman/src/adapter/serialPort.ts (close/error events)
//            .upstream/zigbee-herdsman/src/adapter/z-stack/znp/znp.ts (pipe/parser)
//
// Serial transport actor for the ZNP path. It replaces node-serialport's
// event-driven `pipe`/`on("parsed")`/`on("close")` with a bounded async read
// loop that forwards classified events to the bridge event loop.

use std::io;
use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use super::frame::Frame;
use super::parser::Parser;

/// Events emitted by the serial transport actor.
#[derive(Debug)]
pub enum PortEvent {
    /// One or more complete UNPI frames were parsed.
    Frames(Vec<Frame>),
    /// The port reached EOF (upstream `onPortClose`).
    Closed,
    /// An I/O error occurred. `stops_actor` only means this transport actor
    /// terminated; the supervisor/event loop may reopen the port later (USB
    /// devices can disappear and re-appear).
    Error { error: io::Error, stops_actor: bool },
    /// No data arrived within the poll interval (not an error).
    Idle,
}

/// Upstream `serialPort.on("close")` / `on("error")` classification.
/// EOF is handled separately as [`PortEvent::Closed`].
/// Upstream `serialPort.on("error")` classification.
///
/// Returns `true` only when the current transport actor cannot continue. It is
/// not a decision to stop the application: the supervisor may reopen the port
/// (e.g. an USB adapter disappearing and re-appearing).
pub fn stops_actor(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::BrokenPipe
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::NotConnected
            | io::ErrorKind::NotFound
            | io::ErrorKind::PermissionDenied
            | io::ErrorKind::UnexpectedEof
    )
}

/// Errors that only mean "no data right now" and must not stop the actor.
pub fn is_transient(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
    )
}

/// Open a serial port and run the transport actor until EOF, a fatal error, or
/// the receiver being dropped.
pub async fn run_serial_transport(
    path: impl AsRef<Path>,
    baud_rate: u32,
    poll_interval: Duration,
    tx: mpsc::Sender<PortEvent>,
) -> io::Result<()> {
    let port = serial2_tokio::SerialPort::open(path, baud_rate)?;
    run_transport(port, poll_interval, tx).await
}

/// Run the transport actor over an already opened port.
pub async fn run_transport(
    mut port: serial2_tokio::SerialPort,
    poll_interval: Duration,
    tx: mpsc::Sender<PortEvent>,
) -> io::Result<()> {
    let mut parser = Parser::new();
    let mut buffer = [0u8; 256];

    loop {
        // `select!` makes the actor stop promptly when the bridge event loop
        // drops the receiver, even if `read` is currently pending. `read` is
        // cancel-safe for tokio's AsyncRead contract.
        let read_result = tokio::select! {
            biased;
            _ = tx.closed() => return Ok(()),
            result = tokio::time::timeout(poll_interval, port.read(&mut buffer)) => result,
        };

        match read_result {
            Ok(Ok(0)) => {
                let _ = tx.send(PortEvent::Closed).await;
                return Ok(());
            }
            Ok(Ok(read)) => {
                let frames = parser.transform(&buffer[..read]);
                if !frames.is_empty() && tx.send(PortEvent::Frames(frames)).await.is_err() {
                    return Ok(());
                }
            }
            Ok(Err(error)) => {
                if is_transient(&error) {
                    // WouldBlock is normally handled by tokio readiness, but if
                    // it surfaces treat it like an idle poll.
                    let _ = tx.send(PortEvent::Idle).await;
                } else if stops_actor(&error) {
                    let kind = error.kind();
                    let message = error.to_string();
                    let _ = tx
                        .send(PortEvent::Error {
                            error,
                            stops_actor: true,
                        })
                        .await;
                    return Err(io::Error::new(kind, message));
                } else {
                    let _ = tx
                        .send(PortEvent::Error {
                            error,
                            stops_actor: false,
                        })
                        .await;
                }
            }
            Err(_) => {
                // Poll interval elapsed without data: normal idle condition.
                let _ = tx.send(PortEvent::Idle).await;
            }
        }
    }
}

/// Upstream `serialPort.writeBuffer`: write a full frame and flush it.
pub async fn write_frame(port: &mut serial2_tokio::SerialPort, frame: &Frame) -> io::Result<()> {
    port.write_all(&frame.to_buffer()).await?;
    port.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::z_stack::unpi::constants::{Subsystem, Type};

    #[test]
    fn classifies_transport_errors() {
        assert!(stops_actor(&io::Error::from(io::ErrorKind::BrokenPipe)));
        assert!(stops_actor(&io::Error::from(
            io::ErrorKind::PermissionDenied
        )));
        assert!(stops_actor(&io::Error::from(io::ErrorKind::NotFound)));
        assert!(!stops_actor(&io::Error::from(io::ErrorKind::TimedOut)));
        assert!(!stops_actor(&io::Error::from(io::ErrorKind::WouldBlock)));
        assert!(is_transient(&io::Error::from(io::ErrorKind::TimedOut)));
        assert!(is_transient(&io::Error::from(io::ErrorKind::WouldBlock)));
        assert!(!is_transient(&io::Error::from(io::ErrorKind::NotFound)));
    }

    #[tokio::test]
    async fn parses_frames_and_stops_when_receiver_drops() {
        let (reader, mut writer) = serial2_tokio::SerialPort::pair().expect("pair");
        let (tx, mut rx) = mpsc::channel(8);
        let task = tokio::spawn(run_transport(reader, Duration::from_millis(10), tx));

        let frame = Frame::new(Type::AREQ, Subsystem::ZDO, 0x80, vec![0x01, 0x02], 0, 0);
        writer
            .write_all(&frame.to_buffer())
            .await
            .expect("write frame");
        writer.flush().await.expect("flush");

        let mut received = None;
        for _ in 0..10 {
            if let Some(PortEvent::Frames(frames)) =
                tokio::time::timeout(Duration::from_millis(500), rx.recv())
                    .await
                    .expect("event timeout")
            {
                received = Some(frames);
                break;
            }
        }
        let frames = received.expect("frames");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].r#type, frame.r#type);
        assert_eq!(frames[0].subsystem, frame.subsystem);
        assert_eq!(frames[0].commandID, frame.commandID);
        assert_eq!(frames[0].data, frame.data);

        // Dropping the event receiver stops the actor cleanly (the bridge loop
        // is the only owner). EOF/error delivery is platform-dependent on PTYs
        // and is covered by `is_fatal` above.
        drop(rx);
        let result = tokio::time::timeout(Duration::from_millis(500), task)
            .await
            .expect("actor did not stop")
            .expect("join");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn stops_while_read_pending_when_receiver_drops() {
        let (reader, _writer) = serial2_tokio::SerialPort::pair().expect("pair");
        let (tx, rx) = mpsc::channel(1);
        let task = tokio::spawn(run_transport(reader, Duration::from_secs(60), tx));
        drop(rx);
        let result = tokio::time::timeout(Duration::from_millis(500), task)
            .await
            .expect("actor did not stop while read was pending")
            .expect("join");
        assert!(result.is_ok());
    }
}
