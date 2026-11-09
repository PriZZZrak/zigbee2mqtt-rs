// Original: .upstream/zigbee-herdsman/src/adapter/transport.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).
//
// Node-only members are not ported: `serialPortBinding` / `serialPortInstance` /
// `socketInstance` (test hooks), `node26SerialWorkaround` (Node runtime bug) and
// `get()` (unused modem-status query; `set()` is ported for the bootloader flow).
//
// A raw serial or TCP connection shared by an adapter's protocol stack.
// Upstream exposes a Node EventEmitter (`data` / `close`); here the reader task
// forwards equivalent events over a bounded channel. Function/variable names
// keep the upstream spelling so future upstream diffs map line by line.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use super::utils::{is_tcp_path, parse_tcp_path, TcpPath};

/// Upstream `TransportOptions`.
#[derive(Debug, Clone, Default)]
pub struct TransportOptions {
    /// Upstream `path`.
    pub path: String,
    /// Upstream `baudRate`.
    pub baud_rate: Option<u32>,
    /// Upstream `rtscts` — transport.ts:52.
    pub rtscts: Option<bool>,
}

impl TransportOptions {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            baud_rate: None,
            rtscts: None,
        }
    }
}

/// Upstream `TransportEventMap` (`data` / `close`).
#[derive(Debug, Clone)]
pub enum TransportEvent {
    /// Upstream `data: [data: Buffer]`.
    Data(Vec<u8>),
    /// Upstream `close: [error?: boolean | Error]`.
    Close(Option<String>),
}

/// Upstream `SetOptions` subset used by the adapter flow-control workaround.
#[derive(Debug, Clone, Copy, Default)]
pub struct ModemBits {
    pub dtr: Option<bool>,
    pub rts: Option<bool>,
}

enum TransportWriter {
    /// Upstream `serialPort`.
    Serial(serial2_tokio::SerialPort),
    /// Upstream `socket`.
    Tcp(tokio::net::tcp::OwnedWriteHalf),
}

/// Upstream `AdapterTransport`.
pub struct AdapterTransport {
    /// Upstream `options`.
    pub options: TransportOptions,
    /// Upstream `opening`.
    opening: bool,
    /// Upstream `closing`.
    closing: bool,
    /// Tracks the live state shared with the reader task.
    is_open: Arc<AtomicBool>,
    writer: Option<TransportWriter>,
    reader: Option<tokio::task::JoinHandle<()>>,
    events: Option<mpsc::Receiver<TransportEvent>>,
}

impl AdapterTransport {
    /// Upstream `constructor(options)` — transport.ts:30.
    pub fn new(options: TransportOptions) -> Self {
        Self {
            options,
            opening: false,
            closing: false,
            is_open: Arc::new(AtomicBool::new(false)),
            writer: None,
            reader: None,
            events: None,
        }
    }

    /// Upstream `isOpen` — transport.ts:36.
    pub fn is_open(&self) -> bool {
        !self.opening
            && !self.closing
            && self.is_open.load(Ordering::SeqCst)
            && self.writer.is_some()
    }

    /// Upstream `isOpening` — transport.ts:40.
    pub fn is_opening(&self) -> bool {
        self.opening
    }

    /// Upstream `isClosing` — transport.ts:44.
    pub fn is_closing(&self) -> bool {
        self.closing
    }

    /// Upstream `isSerial` — transport.ts:48.
    pub fn is_serial(&self) -> bool {
        matches!(self.writer, Some(TransportWriter::Serial(_)))
    }

    /// Upstream `rtscts`.
    pub fn rtscts(&self) -> bool {
        self.options.rtscts.unwrap_or(false)
    }

    /// Take the receiver side of the `data`/`close` event stream.
    pub fn take_events(&mut self) -> Option<mpsc::Receiver<TransportEvent>> {
        self.events.take()
    }

    /// Upstream `open(serialNoFlowCtrl = false)` — transport.ts:64.
    pub async fn open(&mut self, serial_no_flow_ctrl: bool) -> Result<(), String> {
        if self.is_open() {
            self.close().await?;
        }

        if self.options.path.is_empty() {
            return Err("Cannot open transport without a path".to_string());
        }

        self.opening = true;
        let result = if is_tcp_path(&self.options.path) {
            self.open_socket().await
        } else {
            self.open_serial(serial_no_flow_ctrl).await
        };
        self.opening = false;
        result
    }

    /// Upstream `close()` — transport.ts:86.
    pub async fn close(&mut self) -> Result<(), String> {
        if self.closing || self.writer.is_none() {
            return Ok(());
        }

        self.closing = true;
        self.is_open.store(false, Ordering::SeqCst);

        if let Some(reader) = self.reader.take() {
            reader.abort();
        }
        let writer = self.writer.take();
        self.events = None;

        let result = match writer {
            Some(TransportWriter::Serial(mut port)) => {
                port.shutdown().await.map_err(|error| error.to_string())
            }
            Some(TransportWriter::Tcp(mut socket)) => {
                socket.shutdown().await.map_err(|error| error.to_string())
            }
            None => Ok(()),
        };

        self.closing = false;
        result
    }

    /// Upstream `write(data)` — transport.ts:118.
    pub async fn write(&mut self, data: &[u8]) -> Result<(), String> {
        if !self.is_open() {
            return Err("Cannot write while transport is closed".to_string());
        }

        match self.writer.as_mut() {
            Some(TransportWriter::Serial(port)) => {
                port.write_all(data)
                    .await
                    .map_err(|error| error.to_string())?;
                port.flush().await.map_err(|error| error.to_string())
            }
            Some(TransportWriter::Tcp(socket)) => {
                socket
                    .write_all(data)
                    .await
                    .map_err(|error| error.to_string())?;
                socket.flush().await.map_err(|error| error.to_string())
            }
            None => Err("Cannot write while transport is closed".to_string()),
        }
    }

    /// Upstream `set(options)` — transport.ts:130.
    pub async fn set(&mut self, bits: ModemBits) -> Result<(), String> {
        let Some(TransportWriter::Serial(port)) = self.writer.as_ref() else {
            return Err("Cannot set while serial transport is closed".to_string());
        };

        if let Some(dtr) = bits.dtr {
            port.set_dtr(dtr).map_err(|error| error.to_string())?;
        }
        if let Some(rts) = bits.rts {
            port.set_rts(rts).map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    /// Attach an already opened serial port. Used for PTY pairs in tests and by
    /// callers that own the port lifecycle; the reader task starts immediately.
    pub fn attach_serial(&mut self, port: serial2_tokio::SerialPort) -> Result<(), String> {
        let writer = port.try_clone().map_err(|error| error.to_string())?;
        self.start_serial_reader(port);
        self.writer = Some(TransportWriter::Serial(writer));
        self.is_open.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Upstream `openSerial(path, noFlowCtrl)` — transport.ts:154.
    async fn open_serial(&mut self, no_flow_ctrl: bool) -> Result<(), String> {
        use serial2_tokio::{FlowControl, Settings};

        // Upstream `const rtscts = this.options.rtscts ?? false` and
        // `const swFlow = !rtscts && !noFlowCtrl`.
        let rtscts = self.rtscts();
        let sw_flow = !rtscts && !no_flow_ctrl;

        // Upstream `openOptions` (`dataBits: 8`, `parity: "none"`, `stopBits: 1`).
        let settings = |mut settings: Settings| -> std::io::Result<Settings> {
            settings.set_raw();
            settings.set_baud_rate(self.options.baud_rate.unwrap_or(115200))?;
            settings.set_flow_control(if rtscts {
                FlowControl::RtsCts
            } else if sw_flow {
                FlowControl::XonXoff
            } else {
                FlowControl::None
            });
            Ok(settings)
        };

        let port = serial2_tokio::SerialPort::open(&self.options.path, settings)
            .map_err(|error| error.to_string())?;
        let writer = port.try_clone().map_err(|error| error.to_string())?;

        self.start_serial_reader(port);
        self.writer = Some(TransportWriter::Serial(writer));
        self.is_open.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn start_serial_reader(&mut self, mut reader: serial2_tokio::SerialPort) {
        let (tx, rx) = mpsc::channel(32);
        let is_open = self.is_open.clone();
        let task = tokio::spawn(async move {
            let mut buffer = [0u8; 1024];
            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) => {
                        is_open.store(false, Ordering::SeqCst);
                        let _ = tx.send(TransportEvent::Close(None)).await;
                        return;
                    }
                    Ok(read) => {
                        if tx
                            .send(TransportEvent::Data(buffer[..read].to_vec()))
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                    Err(error) => {
                        is_open.store(false, Ordering::SeqCst);
                        let _ = tx
                            .send(TransportEvent::Close(Some(error.to_string())))
                            .await;
                        return;
                    }
                }
            }
        });
        self.events = Some(rx);
        self.reader = Some(task);
    }

    /// Upstream `openSocket(path)` — transport.ts:231.
    async fn open_socket(&mut self) -> Result<(), String> {
        let TcpPath { host, port } = parse_tcp_path(&self.options.path);
        let stream = tokio::net::TcpStream::connect((host.as_str(), port))
            .await
            .map_err(|error| error.to_string())?;

        // Upstream `socket.setNoDelay(true)` and
        // `socket.setKeepAlive(true, 15000)`.
        stream
            .set_nodelay(true)
            .map_err(|error| error.to_string())?;
        let socket_ref = socket2::SockRef::from(&stream);
        let keepalive = socket2::TcpKeepalive::new().with_time(Duration::from_millis(15_000));
        let _ = socket_ref.set_tcp_keepalive(&keepalive);

        let (mut reader, writer) = stream.into_split();
        let (tx, rx) = mpsc::channel(32);
        let is_open = self.is_open.clone();
        let task = tokio::spawn(async move {
            let mut buffer = [0u8; 1024];
            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) => {
                        is_open.store(false, Ordering::SeqCst);
                        let _ = tx.send(TransportEvent::Close(None)).await;
                        return;
                    }
                    Ok(read) => {
                        if tx
                            .send(TransportEvent::Data(buffer[..read].to_vec()))
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                    Err(error) => {
                        is_open.store(false, Ordering::SeqCst);
                        let _ = tx
                            .send(TransportEvent::Close(Some(error.to_string())))
                            .await;
                        return;
                    }
                }
            }
        });

        self.events = Some(rx);
        self.reader = Some(task);
        self.writer = Some(TransportWriter::Tcp(writer));
        self.is_open.store(true, Ordering::SeqCst);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn writes_and_reads_over_a_serial_pair() {
        let (left, mut right) = serial2_tokio::SerialPort::pair().expect("pair");
        let mut transport = AdapterTransport::new(TransportOptions::default());
        transport.attach_serial(left).expect("attach");

        assert!(transport.is_open());
        assert!(transport.is_serial());

        transport.write(&[0x01, 0x02, 0x03]).await.expect("write");

        let mut buffer = [0u8; 3];
        tokio::io::AsyncReadExt::read_exact(&mut right, &mut buffer)
            .await
            .expect("read");
        assert_eq!(buffer, [0x01, 0x02, 0x03]);

        let mut events = transport.take_events().expect("events");
        tokio::io::AsyncWriteExt::write_all(&mut right, &[0x0a])
            .await
            .expect("write");
        match tokio::time::timeout(Duration::from_millis(500), events.recv()).await {
            Ok(Some(TransportEvent::Data(data))) => assert_eq!(data, vec![0x0a]),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn refuses_to_write_while_closed() {
        let mut transport = AdapterTransport::new(TransportOptions::default());
        let error = transport.write(&[0x00]).await.expect_err("closed");
        assert_eq!(error, "Cannot write while transport is closed");
    }
}
