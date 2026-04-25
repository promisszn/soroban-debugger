# Plugin Reload Status Reporting

The Soroban Debugger provides first-class support for tracking the status of plugin hot reloads. This makes it easier for plugin developers to understand whether state was preserved, when a reload happened, and why a reload might have been rejected or rolled back.

## Reload Outcomes

A plugin reload attempt can result in one of three outcomes:
1. **Success**: The plugin was cleanly reloaded. The new library was loaded, and the preserved state was successfully restored.
2. **Failed**: The plugin failed to load the new version (e.g., due to a syntax error, a version mismatch, or an unresolved dependency). The debugger keeps the old version active.
3. **RolledBack**: The new plugin version loaded successfully but panicked or failed during initialization or state restoration (`restore_from_reload`). The debugger unloads the new version and safely restores the previous version.

## Inspecting Reload Status

### 1. TUI Dashboard
When using the full-screen terminal UI (`soroban-debug tui`), plugin reload events are automatically surfaced in the **Diagnostics** pane. 
- Successful reloads appear as `NOTICE` level events, detailing the size of the preserved state.
- Failures and Rollbacks appear as `ERROR` level events with explicit rejection reasons.

### 2. JSON Output
For integration with external tools and scripts, the reload status is captured in the JSON trace output under the `PluginReloadReport` structure:
```json
{
  "plugin": "example-logger",
  "timestamp": "12:34:56",
  "outcome": "success",
  "preserved_state_bytes": 1024,
  "reason": null
}
```

### 3. Log Output
If you run the debugger with standard logging enabled (`--verbose` or `RUST_LOG=info`), reload events will be logged directly to the console:
```
[INFO] Plugin 'example-logger' reloaded successfully. Preserved state: 1024 bytes.
[ERROR] Plugin 'example-logger' reload rolled back. Reason: Failed to deserialize state.
```

## Benefits for Plugin Authors
By exposing the exact state size and precise rollback reasons, developers can more easily debug serialization issues during the `prepare_reload` and `restore_from_reload` phases without needing to step through the core debugger's loader internals.