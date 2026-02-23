pub mod api;
pub mod events;
pub mod loader;
pub mod manifest;
pub mod registry;

pub use api::{
    InspectorPlugin, OutputFormatter, PluginCommand, PluginError, PluginResult,
    PLUGIN_CONSTRUCTOR_SYMBOL,
};
pub use events::{EventContext, ExecutionEvent, StorageOperation};
pub use loader::{LoadedPlugin, PluginLoader};
pub use manifest::{PluginCapabilities, PluginManifest};
pub use registry::{PluginRegistry, PluginStatistics};
