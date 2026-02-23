# Example Logger Plugin

This is an example plugin for the Soroban Debugger that demonstrates the plugin API.

## Features

- Logs all execution events to a file (`~/.soroban-debug/plugin-logs/example-logger.log`)
- Provides custom commands:
  - `log-stats`: Show logging statistics
  - `log-path`: Show the log file path
  - `clear-log`: Clear the log file
- Supports hot-reload

## Building

```bash
cd examples/plugins/example_logger
cargo build --release
```

## Installation

1. Build the plugin (see above)
2. Create the plugin directory:
   ```bash
   mkdir -p ~/.soroban-debug/plugins/example-logger
   ```
3. Copy the plugin files:
   ```bash
   cp target/release/libexample_logger_plugin.dylib ~/.soroban-debug/plugins/example-logger/
   cp plugin.toml ~/.soroban-debug/plugins/example-logger/
   ```

## Usage

Once installed, the plugin will automatically load when you run `soroban-debug`. It will log all execution events to the log file.

To disable plugins:
```bash
export SOROBAN_DEBUG_NO_PLUGINS=1
soroban-debug run --contract ./contract.wasm --function test
```

## Log Format

The log file contains timestamped entries for each event:
```
[1704067200] Plugin initialized. Logging to: "/Users/user/.soroban-debug/plugin-logs/example-logger.log"
[1704067201] BEFORE_CALL: test with args: Some("[]") (depth: 1)
[1704067201] AFTER_CALL: test - SUCCESS (duration: 125.45ms)
[1704067202] Plugin shutting down. Total events processed: 42
```
