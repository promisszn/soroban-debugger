use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Manifest describing a plugin and its capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin name
    pub name: String,

    /// Plugin version (semantic versioning)
    pub version: String,

    /// Plugin description
    pub description: String,

    /// Plugin author
    pub author: String,

    /// Plugin license
    pub license: Option<String>,

    /// Minimum debugger version required
    pub min_debugger_version: Option<String>,

    /// Plugin capabilities
    pub capabilities: PluginCapabilities,

    /// Path to the plugin library (relative to manifest)
    pub library: String,

    /// Plugin dependencies (other plugins this plugin requires)
    pub dependencies: Vec<String>,
}

/// Capabilities that a plugin can provide
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginCapabilities {
    /// Whether the plugin hooks execution events
    pub hooks_execution: bool,

    /// Whether the plugin provides custom CLI commands
    pub provides_commands: bool,

    /// Whether the plugin provides custom output formatters
    pub provides_formatters: bool,

    /// Whether the plugin supports hot-reload
    pub supports_hot_reload: bool,
}

impl PluginManifest {
    /// Load a manifest from a TOML file
    pub fn from_file(path: &PathBuf) -> Result<Self, String> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read manifest file: {}", e))?;

        toml::from_str(&contents).map_err(|e| format!("Failed to parse manifest: {}", e))
    }

    /// Validate the manifest
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Plugin name cannot be empty".to_string());
        }

        if self.version.is_empty() {
            return Err("Plugin version cannot be empty".to_string());
        }

        if self.library.is_empty() {
            return Err("Plugin library path cannot be empty".to_string());
        }

        // Validate semantic versioning
        if !self.is_valid_semver(&self.version) {
            return Err(format!("Invalid semantic version: {}", self.version));
        }

        if let Some(ref min_version) = self.min_debugger_version {
            if !self.is_valid_semver(min_version) {
                return Err(format!("Invalid minimum debugger version: {}", min_version));
            }
        }

        Ok(())
    }

    fn is_valid_semver(&self, version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return false;
        }

        parts.iter().all(|p| p.parse::<u32>().is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_semver() {
        let manifest = PluginManifest {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            description: "test".to_string(),
            author: "test".to_string(),
            license: None,
            min_debugger_version: Some("0.1.0".to_string()),
            capabilities: PluginCapabilities::default(),
            library: "test.so".to_string(),
            dependencies: vec![],
        };

        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_invalid_semver() {
        let manifest = PluginManifest {
            name: "test".to_string(),
            version: "1.0".to_string(),
            description: "test".to_string(),
            author: "test".to_string(),
            license: None,
            min_debugger_version: None,
            capabilities: PluginCapabilities::default(),
            library: "test.so".to_string(),
            dependencies: vec![],
        };

        assert!(manifest.validate().is_err());
    }
}
