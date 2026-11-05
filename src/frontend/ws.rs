//! Minimal RFC 6455 server-side WebSocket implementation.
//!
//! The project deliberately avoids heavyweight HTTP/WebSocket dependencies:
//! Zigbee2MQTT's frontend needs the handshake, text frames, ping/pong and close
//! only. SHA-1 and base64 are small, pure-Rust implementations below (no C).
//!
//! Client frames are unmasked after reading (the protocol requires masking);
//! server frames are never masked.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const WEBSOCKET_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
/// Hard upper bound for a single frame (frontend messages are JSON).
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// `Sec-WebSocket-Accept` for a client key.
#[must_use]
pub fn accept_key(client_key: &str) -> String {
    let mut input = String::with_capacity(client_key.len() + WEBSOCKET_GUID.len());
    input.push_str(client_key);
    input.push_str(WEBSOCKET_GUID);
    base64_encode(&sha1(input.as_bytes()))
}

/// WebSocket SHA-1 (FIPS 180-1). Small enough that a dependency is not worth
/// it for a single handshake hash.
#[must_use]
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let mut message = data.to_vec();
    let bit_len = (data.len() as u64) * 8;
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in message.as_chunks::<64>().0 {
        let mut w = [0u32; 80];
        for (index, word) in chunk.as_chunks::<4>().0.iter().enumerate() {
            w[index] = u32::from_be_bytes(*word);
        }
        for index in 16..80 {
            w[index] = (w[index - 3] ^ w[index - 8] ^ w[index - 14] ^ w[index - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (index, word) in w.iter().enumerate() {
            let (f, k) = match index {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }

    let mut out = [0u8; 20];
    for (index, value) in h.iter().enumerate() {
        out[index * 4..index * 4 + 4].copy_from_slice(&value.to_be_bytes());
    }
    out
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[must_use]
pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(BASE64[(b0 >> 2) as usize] as char);
        out.push(BASE64[((b0 & 0x03) << 4 | b1 >> 4) as usize] as char);
        out.push(if chunk.len() > 1 {
            BASE64[((b1 & 0x0F) << 2 | b2 >> 6) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            BASE64[(b2 & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[derive(Debug, PartialEq, Eq)]
pub enum Frame {
    Text(String),
    Binary(Vec<u8>),
    Close(Option<u16>, String),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
}

fn close_payload(code: u16, reason: &str) -> Vec<u8> {
    let mut payload = code.to_be_bytes().to_vec();
    payload.extend_from_slice(reason.as_bytes());
    payload
}

/// Server text frame.
#[must_use]
pub fn text_frame(text: &str) -> Vec<u8> {
    encode_frame(0x1, text.as_bytes())
}

/// Server close frame.
#[must_use]
pub fn close_frame(code: u16, reason: &str) -> Vec<u8> {
    encode_frame(0x8, &close_payload(code, reason))
}

/// Server pong frame.
#[must_use]
pub fn pong_frame(payload: &[u8]) -> Vec<u8> {
    encode_frame(0xA, payload)
}

fn encode_frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 10);
    frame.push(0x80 | opcode);
    if payload.len() < 126 {
        frame.push(payload.len() as u8);
    } else if payload.len() <= u16::MAX as usize {
        frame.push(126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }
    frame.extend_from_slice(payload);
    frame
}

/// Read one complete frame. `Ok(None)` means the peer closed the stream.
pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> std::io::Result<Option<Frame>> {
    let mut first = [0u8; 1];
    if reader.read_exact(&mut first).await.is_err() {
        return Ok(None);
    }
    let fin = first[0] & 0x80 != 0;
    let opcode = first[0] & 0x0F;
    let mut second = [0u8; 1];
    reader.read_exact(&mut second).await?;
    let masked = second[0] & 0x80 != 0;
    let mut length = u64::from(second[0] & 0x7F);
    if length == 126 {
        let mut value = [0u8; 2];
        reader.read_exact(&mut value).await?;
        length = u64::from(u16::from_be_bytes(value));
    } else if length == 127 {
        let mut value = [0u8; 8];
        reader.read_exact(&mut value).await?;
        length = u64::from_be_bytes(value);
    }
    if length > MAX_FRAME_BYTES as u64 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "WebSocket frame exceeds limit",
        ));
    }
    let mut mask = [0u8; 4];
    if masked {
        reader.read_exact(&mut mask).await?;
    }
    let mut payload = vec![0u8; length as usize];
    reader.read_exact(&mut payload).await?;
    if masked {
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }
    }
    if !fin {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "fragmented WebSocket frames are not supported",
        ));
    }
    Ok(Some(match opcode {
        0x1 => Frame::Text(String::from_utf8_lossy(&payload).into_owned()),
        0x2 => Frame::Binary(payload),
        0x8 => {
            let code = if payload.len() >= 2 {
                Some(u16::from_be_bytes([payload[0], payload[1]]))
            } else {
                None
            };
            let reason = if payload.len() > 2 {
                String::from_utf8_lossy(&payload[2..]).into_owned()
            } else {
                String::new()
            };
            Frame::Close(code, reason)
        }
        0x9 => Frame::Ping(payload),
        0xA => Frame::Pong(payload),
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unsupported WebSocket opcode",
            ))
        }
    }))
}

/// Write an already-encoded frame.
pub async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    frame: &[u8],
) -> std::io::Result<()> {
    writer.write_all(frame).await?;
    writer.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha1_matches_known_vectors() {
        assert_eq!(
            sha1(b""),
            [
                0xda, 0x39, 0xa3, 0xee, 0x5e, 0x6b, 0x4b, 0x0d, 0x32, 0x55, 0xbf, 0xef, 0x95, 0x60,
                0x18, 0x90, 0xaf, 0xd8, 0x07, 0x09
            ]
        );
        assert_eq!(
            sha1(b"abc"),
            [
                0xa9, 0x99, 0x3e, 0x36, 0x47, 0x06, 0x81, 0x6a, 0xba, 0x3e, 0x25, 0x71, 0x78, 0x50,
                0xc2, 0x6c, 0x9c, 0xd0, 0xd8, 0x9d
            ]
        );
    }

    #[test]
    fn rfc6455_accept_key() {
        assert_eq!(
            accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn base64_round_trip_lengths() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"hello world"), "aGVsbG8gd29ybGQ=");
    }

    #[tokio::test]
    async fn masked_client_text_frame_is_decoded() {
        // "hi" masked with 01 02 03 04.
        let raw = [0x81u8, 0x82, 0x01, 0x02, 0x03, 0x04, b'h' ^ 1, b'i' ^ 2];
        let (mut client, mut server) = tokio::io::duplex(64);
        use tokio::io::AsyncWriteExt;
        client.write_all(&raw).await.unwrap();
        let frame = read_frame(&mut server).await.unwrap().unwrap();
        assert_eq!(frame, Frame::Text("hi".into()));
    }

    #[tokio::test]
    async fn oversized_frame_is_rejected() {
        let mut header = vec![0x81u8, 127];
        header.extend_from_slice(&(MAX_FRAME_BYTES as u64 + 1).to_be_bytes());
        let (mut client, mut server) = tokio::io::duplex(64);
        use tokio::io::AsyncWriteExt;
        client.write_all(&header).await.unwrap();
        assert!(read_frame(&mut server).await.is_err());
    }

    #[tokio::test]
    async fn server_frames_round_trip() {
        let (mut client, mut server) = tokio::io::duplex(4096);
        write_frame(&mut server, &text_frame("{}")).await.unwrap();
        write_frame(&mut server, &pong_frame(b"x")).await.unwrap();
        write_frame(&mut server, &close_frame(4401, "Unauthorized"))
            .await
            .unwrap();
        // Client-side decoding is the same routine (server frames are unmasked).
        assert_eq!(
            read_frame(&mut client).await.unwrap().unwrap(),
            Frame::Text("{}".into())
        );
        assert_eq!(
            read_frame(&mut client).await.unwrap().unwrap(),
            Frame::Pong(b"x".to_vec())
        );
        assert_eq!(
            read_frame(&mut client).await.unwrap().unwrap(),
            Frame::Close(Some(4401), "Unauthorized".into())
        );
        drop(server);
        assert!(read_frame(&mut client).await.unwrap().is_none());
    }
}
