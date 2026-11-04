//! ZCL transaction waiters.
//!
//! ZNP/ZCL responses are correlated by network address, endpoint, cluster,
//! transaction sequence number and command. AF_DATA_CONFIRM reports transport
//! delivery separately from the device's ZCL response; a success confirmation
//! is not treated as a completed command.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::oneshot;

use crate::error::{Error, Result};

use super::commands::{AfDataConfirm, AfIncomingMsg};

const GLOBAL_FRAME: u8 = 0x00;
const DEFAULT_RESPONSE: u8 = 0x0B;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZclIncoming {
    pub nwk_addr: u16,
    pub endpoint: u8,
    pub cluster_id: u16,
    pub trans_seq: u8,
    pub frame_type: u8,
    pub direction: u8,
    pub command: u8,
    pub data: Vec<u8>,
}

impl ZclIncoming {
    /// Parse an AF_INCOMING_MSG payload into the fields used for correlation.
    pub fn parse_af(raw: &[u8]) -> Option<Self> {
        let msg = AfIncomingMsg::parse(raw)?;
        let data = msg.data;
        if data.is_empty() {
            return None;
        }
        let control = data[0];
        let frame_type = control & 0x03;
        let direction = (control >> 3) & 0x01;
        let mut pos = 1;
        if control & 0x04 != 0 {
            pos += 2;
        }
        if data.len() < pos + 2 {
            return None;
        }
        Some(Self {
            nwk_addr: msg.src_addr,
            endpoint: msg.src_ep,
            cluster_id: msg.cluster_id,
            trans_seq: data[pos],
            frame_type,
            direction,
            command: data[pos + 1],
            data,
        })
    }

    #[must_use]
    pub fn default_response_status(&self) -> Option<u8> {
        (self.frame_type == GLOBAL_FRAME && self.command == DEFAULT_RESPONSE)
            .then(|| self.data.last().copied())
            .flatten()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZclRequest {
    pub nwk_addr: u16,
    pub endpoint: u8,
    pub cluster_id: u16,
    pub trans_seq: u8,
    pub frame_type: u8,
    pub command: u8,
    pub expected_response: Option<u8>,
    pub expects_default_response: bool,
}

impl ZclRequest {
    /// Derive the waiter key from the outgoing ZCL payload.
    pub fn from_payload(
        nwk_addr: u16,
        endpoint: u8,
        cluster_id: u16,
        payload: &[u8],
    ) -> Option<Self> {
        if payload.is_empty() {
            return None;
        }
        let control = payload[0];
        let frame_type = control & 0x03;
        let direction = (control >> 3) & 0x01;
        if direction != 0 {
            return None;
        }
        let disable_default = control & 0x10 != 0;
        let mut pos = 1;
        if control & 0x04 != 0 {
            pos += 2;
        }
        if payload.len() < pos + 2 {
            return None;
        }
        let trans_seq = payload[pos];
        let command = payload[pos + 1];
        let expected_response = if frame_type == GLOBAL_FRAME {
            match command {
                0x00 => Some(0x01), // read -> read response
                0x02 => Some(0x04), // write -> write response
                0x03 => Some(0x05), // write undivided -> write response
                0x06 => Some(0x07), // configure reporting -> response
                0x08 => Some(0x09), // read reporting config -> response
                _ => None,
            }
        } else {
            None
        };
        Some(Self {
            nwk_addr,
            endpoint,
            cluster_id,
            trans_seq,
            frame_type,
            command,
            expected_response,
            expects_default_response: !disable_default,
        })
    }

    #[must_use]
    pub fn expects_response(&self) -> bool {
        self.expected_response.is_some() || self.expects_default_response
    }

    fn matches(&self, incoming: &ZclIncoming) -> bool {
        if self.nwk_addr != incoming.nwk_addr
            || self.endpoint != incoming.endpoint
            || self.cluster_id != incoming.cluster_id
            || self.trans_seq != incoming.trans_seq
            || incoming.direction != 1
        {
            return false;
        }
        let command_matches = self
            .expected_response
            .is_some_and(|expected| incoming.command == expected);
        let default_matches = incoming.frame_type == GLOBAL_FRAME
            && incoming.command == DEFAULT_RESPONSE
            && (self.expected_response.is_some() || self.expects_default_response);
        command_matches || default_matches
    }
}

struct Waiter {
    id: u64,
    request: ZclRequest,
    deadline: Instant,
    reply: oneshot::Sender<Result<ZclIncoming>>,
}

#[derive(Default)]
pub struct ZclWaiters {
    next_id: u64,
    waiters: HashMap<u64, Waiter>,
}

impl ZclWaiters {
    pub fn register(
        &mut self,
        request: ZclRequest,
        timeout: Duration,
    ) -> (u64, oneshot::Receiver<Result<ZclIncoming>>) {
        self.next_id = self.next_id.wrapping_add(1);
        let id = self.next_id;
        let (reply, receiver) = oneshot::channel();
        self.waiters.insert(
            id,
            Waiter {
                id,
                request,
                deadline: Instant::now() + timeout,
                reply,
            },
        );
        (id, receiver)
    }

    /// Deliver a response to every matching waiter. Returns the count.
    pub fn deliver(&mut self, incoming: &ZclIncoming) -> usize {
        let ids: Vec<u64> = self
            .waiters
            .values()
            .filter(|waiter| waiter.request.matches(incoming))
            .map(|waiter| waiter.id)
            .collect();
        for id in &ids {
            if let Some(waiter) = self.waiters.remove(id) {
                let _ = waiter.reply.send(Ok(incoming.clone()));
            }
        }
        ids.len()
    }

    /// Fail waiters whose transaction is reported as undeliverable by
    /// AF_DATA_CONFIRM. Successful confirms do not complete a command.
    pub fn confirm(&mut self, confirm: &AfDataConfirm) -> usize {
        if confirm.status == 0 {
            return 0;
        }
        let ids: Vec<u64> = self
            .waiters
            .values()
            .filter(|waiter| waiter.request.trans_seq == confirm.trans_seq)
            .map(|waiter| waiter.id)
            .collect();
        for id in &ids {
            if let Some(waiter) = self.waiters.remove(id) {
                let _ = waiter.reply.send(Err(Error::Znp(format!(
                    "AF_DATA_CONFIRM failed with status 0x{:02X} for TSN {}",
                    confirm.status, confirm.trans_seq
                ))));
            }
        }
        ids.len()
    }

    /// Remove a waiter, returning whether it was still pending.
    pub fn remove(&mut self, id: u64) -> bool {
        self.waiters.remove(&id).is_some()
    }

    /// Fail waiters whose deadline has passed. Returns the count.
    pub fn expire(&mut self, now: Instant) -> usize {
        let ids: Vec<u64> = self
            .waiters
            .values()
            .filter(|waiter| waiter.deadline <= now)
            .map(|waiter| waiter.id)
            .collect();
        for id in &ids {
            if let Some(waiter) = self.waiters.remove(id) {
                let _ = waiter.reply.send(Err(Error::Timeout));
            }
        }
        ids.len()
    }

    #[must_use]
    pub fn pending(&self) -> usize {
        self.waiters.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NWK: u16 = 0x1234;
    const EP: u8 = 1;
    const CLUSTER: u16 = 0x0402;
    const TSN: u8 = 7;

    fn read_payload(tsn: u8) -> Vec<u8> {
        // global frame, client->server, disable default response not set
        vec![0x00, tsn, 0x00, 0x00, 0x00]
    }

    fn request() -> ZclRequest {
        ZclRequest::from_payload(NWK, EP, CLUSTER, &read_payload(TSN)).unwrap()
    }

    fn incoming(command: u8, endpoint: u8, cluster: u16, tsn: u8, direction: u8) -> ZclIncoming {
        ZclIncoming {
            nwk_addr: NWK,
            endpoint,
            cluster_id: cluster,
            trans_seq: tsn,
            frame_type: GLOBAL_FRAME,
            direction,
            command,
            data: vec![0x18, tsn, command, 0x00],
        }
    }

    #[tokio::test]
    async fn matching_read_response_resolves_waiter() {
        let mut waiters = ZclWaiters::default();
        let (_, rx) = waiters.register(request(), Duration::from_secs(1));
        let response = incoming(0x01, EP, CLUSTER, TSN, 1);
        assert_eq!(waiters.deliver(&response), 1);
        assert_eq!(rx.await.unwrap().unwrap().command, 0x01);
        assert_eq!(waiters.pending(), 0);
    }

    #[test]
    fn wrong_address_endpoint_cluster_tsn_or_direction_is_ignored() {
        let mut waiters = ZclWaiters::default();
        let (_, _rx) = waiters.register(request(), Duration::from_secs(1));
        assert_eq!(waiters.deliver(&incoming(0x01, EP, CLUSTER, TSN + 1, 1)), 0);
        assert_eq!(waiters.deliver(&incoming(0x01, EP + 1, CLUSTER, TSN, 1)), 0);
        assert_eq!(waiters.deliver(&incoming(0x01, EP, CLUSTER + 1, TSN, 1)), 0);
        assert_eq!(waiters.deliver(&incoming(0x01, EP, CLUSTER, TSN, 0)), 0);
        let mut other_nwk = incoming(0x01, EP, CLUSTER, TSN, 1);
        other_nwk.nwk_addr += 1;
        assert_eq!(waiters.deliver(&other_nwk), 0);
        assert_eq!(waiters.pending(), 1);
    }

    #[test]
    fn default_response_is_accepted_and_status_exposed() {
        let mut waiters = ZclWaiters::default();
        let (_, mut rx) = waiters.register(request(), Duration::from_secs(1));
        let mut response = incoming(DEFAULT_RESPONSE, EP, CLUSTER, TSN, 1);
        response.data = vec![0x18, TSN, DEFAULT_RESPONSE, 0x01];
        assert_eq!(waiters.deliver(&response), 1);
        let delivered = rx.try_recv().unwrap().unwrap();
        assert_eq!(delivered.default_response_status(), Some(0x01));
    }

    #[test]
    fn disabled_default_response_is_not_matched() {
        // Cluster-specific command without a known response, default response disabled.
        let payload = vec![0x11, TSN, 0x00];
        let request = ZclRequest::from_payload(NWK, EP, CLUSTER, &payload).unwrap();
        assert!(!request.expects_response());
        let mut waiters = ZclWaiters::default();
        let (_, _rx) = waiters.register(request, Duration::from_secs(1));
        assert_eq!(
            waiters.deliver(&incoming(DEFAULT_RESPONSE, EP, CLUSTER, TSN, 1)),
            0
        );
    }

    #[test]
    fn failed_af_confirm_errors_matching_waiter_but_success_does_not() {
        let mut waiters = ZclWaiters::default();
        let (_, mut rx) = waiters.register(request(), Duration::from_secs(1));
        let success = AfDataConfirm {
            status: 0,
            endpoint: EP,
            trans_seq: TSN,
        };
        assert_eq!(waiters.confirm(&success), 0);
        let failure = AfDataConfirm {
            status: 0xE9,
            endpoint: EP,
            trans_seq: TSN,
        };
        assert_eq!(waiters.confirm(&failure), 1);
        let error = rx.try_recv().unwrap().unwrap_err();
        assert!(error.to_string().contains("0xE9"));
    }

    #[test]
    fn expiry_fails_only_overdue_waiters() {
        let mut waiters = ZclWaiters::default();
        let (_, mut expired_rx) = waiters.register(request(), Duration::from_millis(1));
        let (_, _live_rx) = waiters.register(
            ZclRequest {
                trans_seq: TSN + 1,
                ..request()
            },
            Duration::from_secs(60),
        );
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(waiters.expire(Instant::now()), 1);
        assert!(matches!(
            expired_rx.try_recv().unwrap(),
            Err(Error::Timeout)
        ));
        assert_eq!(waiters.pending(), 1);
    }

    #[test]
    fn parses_zcl_fields_from_af_incoming_frame() {
        let mut raw = vec![0u8; 17];
        raw[2..4].copy_from_slice(&CLUSTER.to_le_bytes());
        raw[4..6].copy_from_slice(&NWK.to_le_bytes());
        raw[6] = EP;
        raw[7] = 1;
        raw[15] = 99;
        // response: global, server->client, TSN 7, readRsp
        let payload = vec![0x18, TSN, 0x01, 0x00, 0x00, 0x00, 0x20, 0x01];
        raw[16] = payload.len() as u8;
        raw.extend(payload);
        let parsed = ZclIncoming::parse_af(&raw).unwrap();
        assert_eq!(parsed.nwk_addr, NWK);
        assert_eq!(parsed.endpoint, EP);
        assert_eq!(parsed.cluster_id, CLUSTER);
        assert_eq!(parsed.trans_seq, TSN);
        assert_eq!(parsed.direction, 1);
        assert_eq!(parsed.command, 0x01);
    }
}
