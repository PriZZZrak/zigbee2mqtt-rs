// Original: .upstream/zigbee-herdsman/src/controller/model/zigbeeEntity.ts
// Upstream revision: zigbee-herdsman v11.2.0 (c1a54e3d).

use std::future::Future;
use std::pin::Pin;

use serde_json::Value;

type EntityFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'a>>;

/// Upstream `ZigbeeEntity` abstract methods — zigbeeEntity.ts:13.
pub trait ZigbeeEntity {
    /// Upstream `read(clusterKey, attributes, options?)`.
    fn read<'a>(
        &'a mut self,
        cluster_key: Value,
        attributes: Value,
        options: Option<Value>,
    ) -> EntityFuture<'a, Option<Value>>;

    /// Upstream `write(clusterKey, attributes, options?)`.
    fn write<'a>(
        &'a mut self,
        cluster_key: Value,
        attributes: Value,
        options: Option<Value>,
    ) -> EntityFuture<'a, ()>;

    /// Upstream `command(clusterKey, commandKey, payload, options?)`.
    fn command<'a>(
        &'a mut self,
        cluster_key: Value,
        command_key: Value,
        payload: Value,
        options: Option<Value>,
    ) -> EntityFuture<'a, Option<Value>>;
}
