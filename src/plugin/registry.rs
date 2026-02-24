use super::api::{PluginError, PluginResult};
use super::events::{EventContext, ExecutionEvent};
use super::loader::{LoadedPlugin, PluginLoader};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tracing::{debug, error, info, warn};

/// Registry that manages all loaded plugins
pub struct PluginRegistry {
    /// Loaded plugins indexed by name
    plugins: HashMap<String, Arc<RwLock<LoadedPlugin>>>,

    /// Plugin loader
    loader: PluginLoader,

    /// Whether hot-reload is enabled
    hot_reload_enabled: bool,
}

impl PluginRegistry {
    /// Create a new plugin registry with the default plugin directory
    pub fn new() -> PluginResult<Self> {
        let plugin_dir = PluginLoader::default_plugin_dir()?;
        Self::with_plugin_dir(plugin_dir)
    }

    /// Create a new plugin registry with a custom plugin directory
    pub fn with_plugin_dir(plugin_dir: PathBuf) -> PluginResult<Self> {
        // Ensure plugin directory exists
        if !plugin_dir.exists() {
            info!("Creating plugin directory: {:?}", plugin_dir);
            std::fs::create_dir_all(&plugin_dir).map_err(|e| {
                PluginError::InitializationFailed(format!(
                    "Failed to create plugin directory: {}",
                    e
                ))
            })?;
        }

        Ok(Self {
            plugins: HashMap::new(),
            loader: PluginLoader::new(plugin_dir),
            hot_reload_enabled: false,
        })
    }

    /// Enable hot-reload functionality
    pub fn enable_hot_reload(&mut self) {
        self.hot_reload_enabled = true;
        info!("Plugin hot-reload enabled");
    }

    /// Disable hot-reload functionality
    pub fn disable_hot_reload(&mut self) {
        self.hot_reload_enabled = false;
        info!("Plugin hot-reload disabled");
    }

    /// Load all plugins from the plugin directory
    pub fn load_all_plugins(&mut self) -> Vec<PluginResult<()>> {
        info!("Loading all plugins from plugin directory");

        let results = self.loader.load_all();
        let mut load_results = Vec::new();

        for result in results {
            match result {
                Ok(plugin) => {
                    let name = plugin.manifest().name.clone();
                    match self.register_plugin(plugin) {
                        Ok(_) => {
                            info!("Successfully registered plugin: {}", name);
                            load_results.push(Ok(()));
                        }
                        Err(e) => {
                            error!("Failed to register plugin {}: {}", name, e);
                            load_results.push(Err(e));
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to load plugin: {}", e);
                    load_results.push(Err(e));
                }
            }
        }

        info!("Loaded {} plugins successfully", self.plugins.len());
        load_results
    }

    /// Register a loaded plugin
    fn register_plugin(&mut self, plugin: LoadedPlugin) -> PluginResult<()> {
        let name = plugin.manifest().name.clone();

        // Check for duplicates
        if self.plugins.contains_key(&name) {
            return Err(PluginError::Invalid(format!(
                "Plugin with name '{}' is already registered",
                name
            )));
        }

        // Check dependencies
        for dep in &plugin.manifest().dependencies {
            if !self.plugins.contains_key(dep) {
                return Err(PluginError::DependencyError(format!(
                    "Plugin '{}' requires plugin '{}' which is not loaded",
                    name, dep
                )));
            }
        }

        self.plugins
            .insert(name.clone(), Arc::new(RwLock::new(plugin)));
        Ok(())
    }

    /// Get a plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<Arc<RwLock<LoadedPlugin>>> {
        self.plugins.get(name).cloned()
    }

    /// Get all plugin names
    pub fn plugin_names(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Get the number of loaded plugins
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Dispatch an event to all plugins
    pub fn dispatch_event(&self, event: &ExecutionEvent, context: &mut EventContext) {
        debug!("Dispatching event to {} plugins", self.plugins.len());

        for (name, plugin_arc) in &self.plugins {
            if let Ok(mut plugin) = plugin_arc.write() {
                if plugin.manifest().capabilities.hooks_execution {
                    match plugin.plugin_mut().on_event(event, context) {
                        Ok(_) => debug!("Plugin '{}' handled event successfully", name),
                        Err(e) => warn!("Plugin '{}' error handling event: {}", name, e),
                    }
                }
            } else {
                warn!("Failed to acquire write lock for plugin '{}'", name);
            }
        }
    }

    /// Reload a specific plugin
    pub fn reload_plugin(&mut self, name: &str) -> PluginResult<()> {
        if !self.hot_reload_enabled {
            return Err(PluginError::ExecutionFailed(
                "Hot-reload is not enabled".to_string(),
            ));
        }

        let plugin_arc = self
            .plugins
            .get(name)
            .ok_or_else(|| PluginError::NotFound(format!("Plugin '{}' not found", name)))?
            .clone();

        // Get plugin info before unloading
        let (manifest_path, saved_state) = {
            let plugin = plugin_arc.write().map_err(|_| {
                PluginError::ExecutionFailed("Failed to acquire plugin lock".to_string())
            })?;

            if !plugin.plugin().supports_hot_reload() {
                return Err(PluginError::ExecutionFailed(format!(
                    "Plugin '{}' does not support hot-reload",
                    name
                )));
            }

            let manifest_path = plugin
                .path()
                .parent()
                .ok_or_else(|| PluginError::Invalid("Invalid plugin path".to_string()))?
                .join("plugin.toml");

            let state = plugin.plugin().prepare_reload().map_err(|e| {
                PluginError::ExecutionFailed(format!("Failed to prepare plugin for reload: {}", e))
            })?;

            (manifest_path, state)
        };

        // Remove old plugin
        self.plugins.remove(name);

        // Load new version
        match self.loader.load_from_manifest(&manifest_path) {
            Ok(mut new_plugin) => {
                // Restore state
                if let Err(e) = new_plugin.plugin_mut().restore_from_reload(saved_state) {
                    error!("Failed to restore plugin state: {}", e);
                }

                self.register_plugin(new_plugin)?;
                info!("Successfully reloaded plugin: {}", name);
                Ok(())
            }
            Err(e) => {
                error!("Failed to reload plugin '{}': {}", name, e);
                Err(e)
            }
        }
    }

    /// Unload all plugins
    pub fn unload_all(&mut self) {
        info!("Unloading all plugins");
        self.plugins.clear();
    }

    /// Get plugin statistics
    pub fn statistics(&self) -> PluginStatistics {
        let mut stats = PluginStatistics::default();

        for plugin_arc in self.plugins.values() {
            if let Ok(plugin) = plugin_arc.read() {
                let caps = &plugin.manifest().capabilities;

                if caps.hooks_execution {
                    stats.hooks_execution += 1;
                }
                if caps.provides_commands {
                    stats.provides_commands += 1;
                }
                if caps.provides_formatters {
                    stats.provides_formatters += 1;
                }
                if caps.supports_hot_reload {
                    stats.supports_hot_reload += 1;
                }
            }
        }

        stats.total = self.plugins.len();
        stats
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new().expect("Failed to create default plugin registry")
    }
}

impl Drop for PluginRegistry {
    fn drop(&mut self) {
        self.unload_all();
    }
}

/// Statistics about loaded plugins
#[derive(Debug, Default, Clone)]
pub struct PluginStatistics {
    pub total: usize,
    pub hooks_execution: usize,
    pub provides_commands: usize,
    pub provides_formatters: usize,
    pub supports_hot_reload: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let temp_dir = std::env::temp_dir().join("soroban-debug-test-plugins");
        let registry = PluginRegistry::with_plugin_dir(temp_dir.clone());
        assert!(registry.is_ok());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_plugin_statistics() {
        let temp_dir = std::env::temp_dir().join("soroban-debug-test-plugins-stats");
        let registry = PluginRegistry::with_plugin_dir(temp_dir.clone()).unwrap();

        let stats = registry.statistics();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.hooks_execution, 0);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
