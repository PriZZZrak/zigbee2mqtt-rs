// Original: .upstream/zigbee-herdsman/src/controller/model/entity.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).
//
// Upstream keeps the database and adapter in static class fields injected by
// the controller. Rust has no class inheritance, so the same process-wide
// injection lives in this module; models call `Entity::database()` /
// `Entity::adapter()`.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use serde_json::Value;

use crate::adapter::events::ZclPayload;
use crate::controller::database::Database;
use crate::zspec::zcl::zcl_frame::Frame;

type AdapterFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'a>>;

/// Controller-facing adapter surface used by the model layer. It mirrors the
/// upstream `Adapter` methods the models call and is implemented by the
/// concrete adapter wrapper.
pub trait ControllerAdapter: Send + Sync {
    /// Upstream `hasZdoMessageOverhead`.
    fn has_zdo_message_overhead(&self) -> bool;

    /// Upstream `sendZdo(ieeeAddress, networkAddress, clusterId, payload, disableResponse)`.
    fn send_zdo(
        &self,
        ieee_address: &str,
        network_address: u16,
        cluster_id: u16,
        payload: &[u8],
        disable_response: bool,
    ) -> AdapterFuture<'_, Value>;

    /// Upstream `sendZclFrameToEndpoint(...)`.
    #[allow(clippy::too_many_arguments)]
    fn send_zcl_frame_to_endpoint(
        &self,
        ieee_address: &str,
        network_address: u16,
        endpoint: u8,
        frame: &Frame,
        timeout: u64,
        disable_response: bool,
        disable_recovery: bool,
        source_endpoint: Option<u8>,
        profile_id: Option<u16>,
    ) -> AdapterFuture<'_, Option<ZclPayload>>;

    /// Upstream `sendZclFrameToGroup(...)`.
    fn send_zcl_frame_to_group(
        &self,
        group_id: u16,
        frame: &Frame,
        source_endpoint: Option<u8>,
        profile_id: Option<u16>,
    ) -> AdapterFuture<'_, ()>;

    /// Upstream `sendZclFrameToAll(...)`.
    fn send_zcl_frame_to_all(
        &self,
        endpoint: u8,
        frame: &Frame,
        source_endpoint: u8,
        destination: u16,
        profile_id: Option<u16>,
    ) -> AdapterFuture<'_, ()>;
}

static DATABASE: OnceLock<RwLock<Option<Arc<Mutex<Database>>>>> = OnceLock::new();
static ADAPTER: OnceLock<RwLock<Option<Arc<dyn ControllerAdapter>>>> = OnceLock::new();

fn database_slot() -> &'static RwLock<Option<Arc<Mutex<Database>>>> {
    DATABASE.get_or_init(|| RwLock::new(None))
}

fn adapter_slot() -> &'static RwLock<Option<Arc<dyn ControllerAdapter>>> {
    ADAPTER.get_or_init(|| RwLock::new(None))
}

/// Upstream `Entity.injectDatabase(database)` — entity.ts:16.
pub fn inject_database(database: Arc<Mutex<Database>>) {
    *database_slot().write().expect("database slot poisoned") = Some(database);
}

/// Upstream `Entity.database`.
pub fn database() -> Arc<Mutex<Database>> {
    database_slot()
        .read()
        .expect("database slot poisoned")
        .clone()
        .expect("Entity.database was not injected")
}

/// Upstream `Entity.injectAdapter(adapter)` — entity.ts:20.
pub fn inject_adapter(adapter: Arc<dyn ControllerAdapter>) {
    *adapter_slot().write().expect("adapter slot poisoned") = Some(adapter);
}

/// Upstream `Entity.adapter`.
pub fn adapter() -> Arc<dyn ControllerAdapter> {
    adapter_slot()
        .read()
        .expect("adapter slot poisoned")
        .clone()
        .expect("Entity.adapter was not injected")
}

/// Upstream `Entity` — marker mirroring the static injection entry points.
pub struct Entity;

impl Entity {
    pub fn inject_database(database: Arc<Mutex<Database>>) {
        inject_database(database);
    }

    pub fn database() -> Arc<Mutex<Database>> {
        database()
    }

    pub fn inject_adapter(adapter: Arc<dyn ControllerAdapter>) {
        inject_adapter(adapter);
    }

    pub fn adapter() -> Arc<dyn ControllerAdapter> {
        adapter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::events::ZclPayload;
    use crate::zspec::zcl::zcl_frame::Frame;
    use serde_json::json;

    struct FakeAdapter;

    impl ControllerAdapter for FakeAdapter {
        fn has_zdo_message_overhead(&self) -> bool {
            false
        }

        fn send_zdo(
            &self,
            _ieee_address: &str,
            _network_address: u16,
            _cluster_id: u16,
            _payload: &[u8],
            _disable_response: bool,
        ) -> AdapterFuture<'_, Value> {
            Box::pin(async move { Ok(json!([0, {"ok": true}])) })
        }

        fn send_zcl_frame_to_endpoint(
            &self,
            _ieee_address: &str,
            _network_address: u16,
            _endpoint: u8,
            _frame: &Frame,
            _timeout: u64,
            _disable_response: bool,
            _disable_recovery: bool,
            _source_endpoint: Option<u8>,
            _profile_id: Option<u16>,
        ) -> AdapterFuture<'_, Option<ZclPayload>> {
            Box::pin(async move { Ok(None) })
        }

        fn send_zcl_frame_to_group(
            &self,
            _group_id: u16,
            _frame: &Frame,
            _source_endpoint: Option<u8>,
            _profile_id: Option<u16>,
        ) -> AdapterFuture<'_, ()> {
            Box::pin(async move { Ok(()) })
        }

        fn send_zcl_frame_to_all(
            &self,
            _endpoint: u8,
            _frame: &Frame,
            _source_endpoint: u8,
            _destination: u16,
            _profile_id: Option<u16>,
        ) -> AdapterFuture<'_, ()> {
            Box::pin(async move { Ok(()) })
        }
    }

    #[tokio::test]
    async fn injection_roundtrip() {
        let database = Arc::new(Mutex::new(Database::open(
            "/tmp/opencode/controller-entity-test.db",
        )));
        Entity::inject_database(database.clone());
        Entity::inject_adapter(Arc::new(FakeAdapter));

        assert!(Arc::ptr_eq(&Entity::database(), &database));
        assert!(!Entity::adapter().has_zdo_message_overhead());
        let response = Entity::adapter()
            .send_zdo("0x00124b0000000001", 0x1234, 0x0002, &[0], false)
            .await
            .expect("send_zdo");
        assert_eq!(response[0], json!(0));
        assert_eq!(response[1]["ok"], json!(true));
    }
}
