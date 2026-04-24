# Soroban Debugger Starter Plugin

This is a template to help you quickly build your own plugins for the Soroban Debugger.

## Getting Started

1. Copy this directory to use as the base for your new plugin.
2. Rename the package in `Cargo.toml`.
3. Update the `plugin.toml` manifest with your plugin's details.
4. Update the `[capabilities]` section depending on whether you need to hook execution, provide custom CLI commands, or use formatters.

## Building

```bash
cargo build --release
```

## Installing

Create a directory for your plugin inside the debugger's plugin directory:

```bash
mkdir -p ~/.soroban-debug/plugins/starter-plugin
```

Copy the compiled dynamic library and the manifest (adjusting the library extension for your OS):

```bash
cp target/release/libsoroban_debug_starter_plugin.so ~/.soroban-debug/plugins/starter-plugin/
cp plugin.toml ~/.soroban-debug/plugins/starter-plugin/
```

## Running

Once installed, the Soroban Debugger will automatically discover and load your plugin on startup.
Because of the default trust policy, you may need to add it to your allowlist or run in local-only mode:

```bash
SOROBAN_DEBUG_PLUGIN_ALLOWLIST="starter-plugin" soroban-debug run --contract my_contract.wasm --function test
```