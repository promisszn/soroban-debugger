# Plugin System Architecture

The Soroban Debugger features a dynamic plugin system that allows developers to extend its functionality at runtime without modifying the core debugger codebase.

## Subsystem Overview

The plugin architecture relies on dynamic library loading (`.so`, `.dylib`, `.dll`) to inject custom logic into the debugger's execution lifecycle.

### Key Components

1. **Plugin Registry**: The core component responsible for discovering, validating, loading, and managing plugins.
2. **`InspectorPlugin` Trait**: The Rust trait that all plugins must implement to interact with the debugger.
3. **Execution Events**: A standardized set of events (e.g., `BeforeFunctionCall`, `StorageAccess`) dispatched by the `DebuggerEngine` to the `PluginRegistry`.

## Plugin Lifecycle

```mermaid
graph TD
    Start[Debugger Startup] --> Discover[Scan ~/.soroban-debug/plugins/]
    Discover --> Manifest[Parse plugin.toml]
    Manifest --> Policy[Evaluate Trust Policy]
    Policy --> Load[Load Dynamic Library]
    Load --> Init[Call initialize()]
    Init --> Active[Plugin Active]
    
    Active -->|Execution Event| EventHook[Call on_event()]
    Active -->|User Command| CmdHook[Call execute_command()]
    Active -->|Hot-Reload| Reload[Prepare & Restore State]
    
    Active --> Shutdown[Debugger Shutdown]
    Shutdown --> Clean[Call shutdown()]
```

## Integration Points

Plugins interact with the debugger in three primary ways:

1. **Event Hooks**: The `DebuggerEngine` dispatches `ExecutionEvent`s to the `PluginRegistry`, which broadcasts them to all loaded plugins. Plugins can inspect state, log information, or modify the shared `EventContext`.
2. **Custom Commands**: Plugins can declare custom CLI commands via the `commands()` method. The CLI argument parser treats these as external subcommands and routes them to the appropriate plugin's `execute_command()` method.
3. **Formatters**: Plugins can provide custom formatters for specific data types, allowing them to transform output (e.g., JSON pretty-printing) via the `format_output()` method.

## Hot-Reload Mechanism

The plugin system supports hot-reloading to facilitate iterative development without restarting the debugger:

1. The user triggers a reload.
2. The `PluginRegistry` calls `prepare_reload()` on the active plugin to serialize its state.
3. The old dynamic library is unloaded, and the new dynamic library is loaded.
4. The `PluginRegistry` calls `restore_from_reload()` on the newly instantiated plugin, passing the serialized state.
5. The registry computes a diff of capabilities, commands, and formatters to present a summary of the reload.

## Trust and Security

Since plugins execute as native code within the debugger process, they are subject to a strict trust policy:
- **Off/Warn/Enforce Modes**: Configurable via environment variables.
- **Manifest Signatures**: `plugin.toml` can include Ed25519 signatures to verify the integrity of the plugin library.
- **Allowlists/Denylists**: Specific plugins can be explicitly permitted or blocked by the developer.