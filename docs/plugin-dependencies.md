# Plugin Dependencies

The Soroban Debugger plugin system allows plugins to declare dependencies on other plugins. This is useful for building plugin ecosystems where specialized plugins extend base plugins.

## Dependency Resolution
When loading a plugin with dependencies, the debugger performs a topological sort of the dependency graph to determine the correct load order. If any dependencies are missing, have incompatible versions, or form a circular reference (cycle), the plugin load will fail.

## Dependency Reporting
To aid in debugging plugin load failures, the debugger emits a structured dependency resolution report. This report details the exact load order attempted and pinpoints where the failure occurred.

### Report Format
The dependency report is emitted as part of the JSON output when a plugin fails to load due to dependency issues.

```json
{
  "root_plugin": "advanced-profiler",
  "success": false,
  "load_order": [
    "base-metrics"
  ],
  "nodes": [
    {
      "name": "advanced-profiler",
      "version": "1.2.0",
      "dependencies": [
        "base-metrics"
      ],
      "status": "failed",
      "error": {
        "version_mismatch": {
          "expected": "^1.0.0",
          "found": "0.9.5"
        }
      }
    }
  ]
}
```

### Common Failure Reasons
- **CycleDetected**: A circular dependency was found in the plugin graph.
- **VersionMismatch**: A required dependency was found, but its version did not satisfy the requirement.
- **NotFound**: A required dependency could not be located in the plugin registry.
- **LoadError**: The dependency was found but failed to load (e.g., due to a panic during initialization).