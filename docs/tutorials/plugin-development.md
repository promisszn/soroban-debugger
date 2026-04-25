# Building Your First Soroban Debugger Plugin

This tutorial walks you through writing, building, installing, and iterating on a real debugger plugin end-to-end. By the end you will have a working **gas-spike alerter** plugin that watches every function call and prints a warning whenever CPU instruction usage exceeds a configurable threshold.

For the complete API reference, see [Plugin API](../plugin-api.md). This tutorial focuses on the workflow, not the reference.

## Prerequisites

- Soroban Debugger installed and on your `$PATH`
- Rust toolchain (stable, 1.75+)
- A compiled contract WASM to test against — we'll use `examples/contracts/simple-token`

---

## 1. Understand the Plugin Model

Plugins are Rust `cdylib` crates. The debugger loads them at startup from `~/.soroban-debug/plugins/` using `libloading`. Each plugin:

1. Exports a single C-ABI function `create_plugin()` that returns a boxed `InspectorPlugin` trait object.
2. Provides a `plugin.toml` manifest so the debugger knows its name, version, and capabilities before loading the shared library.
3. Receives `ExecutionEvent` callbacks during contract execution.

```
~/.soroban-debug/plugins/
└── gas-spike-alerter/
    ├── plugin.toml          ← manifest (read first)
    └── libgas_spike_alerter.dylib  ← shared library (loaded second)
```

The full lifecycle is: **discover → validate manifest → trust check → load library → `initialize()` → receive events → `shutdown()`**.

---

## 2. Create the Crate

```bash
cargo new --lib gas-spike-alerter
cd gas-spike-alerter
```

Open `Cargo.toml` and replace its contents:

```toml
[package]
name = "gas-spike-alerter"
version = "0.1.0"
edition = "2021"

[lib]
# cdylib produces a .so / .dylib / .dll that can be dynamically loaded
crate-type = ["cdylib"]

[dependencies]
# Point at your local checkout. Adjust the path as needed.
soroban-debugger = { path = "../../.." }
```

> If you're developing outside the debugger repository, publish the `soroban-debugger` crate to crates.io or use a git dependency instead.

---

## 3. Write the Plugin

Replace `src/lib.rs` with the following. Read the inline comments — they explain every decision.

```rust
use soroban_debugger::plugin::{
    EventContext, ExecutionEvent, InspectorPlugin, PluginCapabilities, PluginManifest,
    PluginResult,
};
use std::any::Any;

// ── Plugin state ──────────────────────────────────────────────────────────────

pub struct GasSpikeAlerter {
    manifest: PluginManifest,
    /// Warn when a single function call exceeds this many CPU instructions.
    threshold: u64,
    /// Running count of alerts fired this session.
    alert_count: usize,
}

impl GasSpikeAlerter {
    fn new() -> Self {
        Self {
            manifest: PluginManifest {
                name: "gas-spike-alerter".to_string(),
                version: "0.1.0".to_string(),
                description: "Warns when a function call exceeds a CPU-instruction threshold"
                    .to_string(),
                author: "Your Name".to_string(),
                license: Some("MIT".to_string()),
                min_debugger_version: Some("0.1.0".to_string()),
                capabilities: PluginCapabilities {
                    hooks_execution: true,
                    provides_commands: false,
                    provides_formatters: false,
                    // We don't carry across-reload state, so hot-reload is trivial.
                    supports_hot_reload: true,
                },
                library: "libgas_spike_alerter.dylib".to_string(),
                dependencies: vec![],
                signature: None,
            },
            // Read threshold from the environment; default to 500 000 instructions.
            threshold: std::env::var("GAS_SPIKE_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(500_000),
            alert_count: 0,
        }
    }
}

// ── Trait implementation ──────────────────────────────────────────────────────

impl InspectorPlugin for GasSpikeAlerter {
    fn metadata(&self) -> PluginManifest {
        self.manifest.clone()
    }

    fn initialize(&mut self) -> PluginResult<()> {
        eprintln!(
            "[gas-spike-alerter] loaded — threshold: {} instructions",
            self.threshold
        );
        Ok(())
    }

    fn shutdown(&mut self) -> PluginResult<()> {
        eprintln!(
            "[gas-spike-alerter] shutdown — {} alert(s) fired this session",
            self.alert_count
        );
        Ok(())
    }

    fn on_event(&mut self, event: &ExecutionEvent, _ctx: &mut EventContext) -> PluginResult<()> {
        // We only care about completed function calls that carry budget information.
        if let ExecutionEvent::AfterFunctionCall {
            function,
            cpu_instructions,
            ..
        } = event
        {
            if let Some(instructions) = cpu_instructions {
                if *instructions > self.threshold {
                    self.alert_count += 1;
                    eprintln!(
                        "[gas-spike-alerter] ⚠️  SPIKE in '{}': {} instructions (threshold: {})",
                        function, instructions, self.threshold
                    );
                }
            }
        }
        Ok(())
    }

    // ── Hot-reload support ────────────────────────────────────────────────────
    // We don't have state that needs preserving across a reload, so these are
    // trivially implemented.

    fn supports_hot_reload(&self) -> bool {
        true
    }

    fn prepare_reload(&self) -> PluginResult<Box<dyn Any + Send>> {
        Ok(Box::new(()))
    }

    fn restore_from_reload(&mut self, _state: Box<dyn Any + Send>) -> PluginResult<()> {
        Ok(())
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// The debugger calls this symbol to obtain a plugin instance.
/// Must be `no_mangle` and `extern "C"` to survive dynamic linking.
#[no_mangle]
pub extern "C" fn create_plugin() -> *mut dyn InspectorPlugin {
    Box::into_raw(Box::new(GasSpikeAlerter::new()))
}
```

### What each piece does

| Part | Purpose |
|---|---|
| `PluginManifest` | Metadata the debugger reads from the in-memory struct (after loading) and from `plugin.toml` (before loading). Keep them in sync. |
| `initialize` / `shutdown` | Lifecycle hooks — good for opening files, logging session summaries. |
| `on_event` | Called for every `ExecutionEvent`. Pattern-match only the variants you care about; ignore the rest. |
| `supports_hot_reload` + `prepare_reload` + `restore_from_reload` | Allow the plugin to be rebuilt and reloaded without restarting the debugger. |
| `create_plugin` | The single C-ABI symbol the loader looks for. It heap-allocates the plugin and hands ownership to the debugger. |

---

## 4. Write the Manifest

Create `plugin.toml` in the crate root (next to `Cargo.toml`):

```toml
schema_version = "1.0.0"
name           = "gas-spike-alerter"
version        = "0.1.0"
description    = "Warns when a function call exceeds a CPU-instruction threshold"
author         = "Your Name"
license        = "MIT"
min_debugger_version = "0.1.0"

[capabilities]
hooks_execution    = true
provides_commands  = false
provides_formatters = false
supports_hot_reload = true

# The filename must match what you produce on your OS:
#   Linux:   libgas_spike_alerter.so
#   macOS:   libgas_spike_alerter.dylib
#   Windows: gas_spike_alerter.dll
library = "libgas_spike_alerter.dylib"

dependencies = []
```

---

## 5. Build the Plugin

```bash
cargo build --release
```

On macOS the output is:

```
target/release/libgas_spike_alerter.dylib
```

On Linux it ends in `.so`; on Windows, `.dll`. Adjust the `library` field in `plugin.toml` (and the manifest in `src/lib.rs`) to match your platform.

---

## 6. Install

```bash
# Create the plugin directory
mkdir -p ~/.soroban-debug/plugins/gas-spike-alerter

# Copy the shared library (adjust extension for your OS)
cp target/release/libgas_spike_alerter.dylib \
   ~/.soroban-debug/plugins/gas-spike-alerter/

# Copy the manifest
cp plugin.toml ~/.soroban-debug/plugins/gas-spike-alerter/
```

---

## 7. Verify the Plugin Loads

Run any debugger command. The plugin's `initialize` message prints to stderr:

```bash
soroban-debugger run \
  --contract path/to/simple_token.wasm \
  --function balance \
  --args '["GD726E62Z6XU6KD5J2EPOHG5NQZ5K5I5J5QZQZQZQZQZQZQZQZQZQZQ"]'
```

Expected output (stderr):

```
[gas-spike-alerter] loaded — threshold: 500000 instructions
```

If you don't see this, check the [plugin loading troubleshooting](#troubleshooting) section at the end.

---

## 8. Test the Alert

Use the `mint` function, which is more computationally intensive:

```bash
soroban-debugger run \
  --contract path/to/simple_token.wasm \
  --function mint \
  --args '["GD726E62Z6XU6KD5J2EPOHG5NQZ5K5I5J5QZQZQZQZQZQZQZQZQZQZQ", 1000000]'
```

To force an alert without waiting for a real spike, lower the threshold:

```bash
GAS_SPIKE_THRESHOLD=1000 soroban-debugger run \
  --contract path/to/simple_token.wasm \
  --function mint \
  --args '["GD726E62Z6XU6KD5J2EPOHG5NQZ5K5I5J5QZQZQZQZQZQZQZQZQZQZQ", 1000000]'
```

Expected stderr:

```
[gas-spike-alerter] loaded — threshold: 1000 instructions
[gas-spike-alerter] ⚠️  SPIKE in 'mint': 42731 instructions (threshold: 1000)
[gas-spike-alerter] shutdown — 1 alert(s) fired this session
```

---

## 9. Iterate with Hot-Reload

Hot-reload lets you recompile and reload the plugin without restarting a long-running debugger session (e.g., in `interactive` or `repl` mode).

### Start an interactive session

```bash
soroban-debugger interactive \
  --contract path/to/simple_token.wasm \
  --function initialize \
  --args '["GD5DJ3...", "My Token", "MTK"]'
```

### Edit the plugin

Change the alert message in `src/lib.rs` (e.g., add `[ALERT]` prefix):

```rust
eprintln!(
    "[ALERT][gas-spike-alerter] ⚠️  SPIKE in '{}': {} instructions (threshold: {})",
    function, instructions, self.threshold
);
```

Bump the version to `"0.1.1"` in both `src/lib.rs` and `plugin.toml`.

### Rebuild

```bash
cargo build --release
cp target/release/libgas_spike_alerter.dylib \
   ~/.soroban-debug/plugins/gas-spike-alerter/
cp plugin.toml ~/.soroban-debug/plugins/gas-spike-alerter/
```

### Trigger the reload

In the interactive session, run:

```
(debugger) plugin reload gas-spike-alerter
```

The debugger reports what changed:

```
Plugin 'gas-spike-alerter' reload changes:
  Version: 0.1.0 → 0.1.1
```

Continue the session — subsequent function calls use the updated plugin immediately.

---

## 10. Add a Custom Command (Optional Extension)

To expose a `spike-summary` command that prints total alert counts on demand, extend the plugin:

```rust
use soroban_debugger::plugin::PluginCommand;

impl InspectorPlugin for GasSpikeAlerter {
    // ... existing methods ...

    fn commands(&self) -> Vec<PluginCommand> {
        vec![PluginCommand {
            name: "spike-summary".to_string(),
            description: "Print the number of gas-spike alerts fired this session".to_string(),
            arguments: vec![],
        }]
    }

    fn execute_command(&mut self, command: &str, _args: &[String]) -> PluginResult<String> {
        match command {
            "spike-summary" => Ok(format!(
                "{} spike alert(s) fired (threshold: {} instructions)",
                self.alert_count, self.threshold
            )),
            _ => Err(soroban_debugger::plugin::PluginError::ExecutionFailed(
                format!("unknown command: {}", command),
            )),
        }
    }
}
```

Update `plugin.toml`:

```toml
[capabilities]
hooks_execution   = true
provides_commands = true   # ← flip this
```

Rebuild and reinstall, then in an interactive session:

```
(debugger) spike-summary
2 spike alert(s) fired (threshold: 500000 instructions)
```

---

## 11. Sign the Plugin for Enforce Mode (Optional)

In CI environments where `SOROBAN_DEBUG_PLUGIN_TRUST_MODE=enforce` is set, unsigned plugins are blocked. To sign:

```bash
# Generate a key pair (only once per team)
soroban-debugger plugin sign \
  --manifest plugin.toml \
  --library libgas_spike_alerter.dylib \
  --key-out team-release.key \
  --pub-out team-release.pub

# The command appends a [signature] block to plugin.toml
```

Tell the debugger to trust your key:

```bash
export SOROBAN_DEBUG_PLUGIN_ALLOWED_SIGNERS=$(cat team-release.pub)
```

See [Plugin API § Trust Policy](../plugin-api.md#trust-policy) for the full trust model.

---

## Troubleshooting

### Plugin not loading

- Confirm `~/.soroban-debug/plugins/gas-spike-alerter/plugin.toml` exists and is valid TOML.
- Run `soroban-debugger run ... 2>&1 | head -20` to see early stderr output.
- Make sure the `library` field in `plugin.toml` matches the actual filename on your OS.
- Verify the shared library exports `create_plugin`:
  ```bash
  # macOS / Linux
  nm -D target/release/libgas_spike_alerter.dylib | grep create_plugin
  ```
- If trust mode is blocking the plugin, either allowlist it or relax the mode:
  ```bash
  SOROBAN_DEBUG_PLUGIN_TRUST_MODE=warn soroban-debugger run ...
  ```

### `on_event` not called

- Confirm `hooks_execution = true` in both `plugin.toml` and `PluginCapabilities` in `src/lib.rs`.
- The `AfterFunctionCall` variant only carries `cpu_instructions` when the debugger was built with budget tracking enabled. Try `--verbose` to confirm budget data is being recorded.

### Hot-reload shows no changes

- Verify you copied the newly built library to the plugin directory before triggering reload.
- Check that the version in `plugin.toml` was bumped — the change-detection diff uses the manifest.

---

## What to Read Next

- [Plugin API Reference](../plugin-api.md) — complete trait documentation, all `ExecutionEvent` variants, and the full manifest schema.
- [Plugin Manifest Versioning](../plugin-manifest-versioning.md) — how to handle breaking changes across debugger versions.
- [Plugin Failure Handling](../plugin-failure-handling.md) — what happens when a plugin panics or returns an error.
- [Plugin Sandbox Policy](../plugin-sandbox-policy.md) — resource and capability limits applied to plugins.
- [Example Logger Plugin](../../examples/plugins/example_logger/) — a fuller example with file I/O and multiple commands.
