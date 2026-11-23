// Converter runtime integration for the Rust bridge.
//
// The runtime executes the pinned upstream JavaScript; the bridge sees ordinary
// `zigbee_converters::Definition` values and keeps its existing Receive,
// Publish, Configure and OnEvent flow.

pub mod dispatcher;
pub mod transport;

pub use dispatcher::{
    device_spec, ConverterDispatcher, DefinitionMetadata, DispatcherConfig, PreparedDefinition,
};
pub use transport::{ConverterTransport, OfflineMockTransport, TransportFuture};
