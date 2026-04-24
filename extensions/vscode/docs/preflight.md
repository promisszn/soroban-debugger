# Soroban Debugger Preflight Diagnostics

The Soroban Debugger VS Code extension includes a "Launch Preflight Check" feature to help identify and resolve configuration issues *before* starting a debug session. This saves time and provides actionable, editor-friendly feedback.

## What is Preflight?

Preflight is an initial validation step that inspects your `launch.json` configuration and your local environment. It ensures that the debugger has everything it needs to start successfully without waiting for the backend to fail.

## Checks Performed

The preflight process currently validates:

1. **Contract Existence**: Verifies that the `.wasm` file specified in `contractPath` exists and is readable.
2. **Snapshot Validity**: If a `snapshotPath` is provided, ensures the file exists.
3. **Argument Types**: Checks that the `args` array in `launch.json` is properly formatted and serializable.
4. **Port Availability**: If a specific `port` is requested for the debugger server, validates that it is within the allowed range (1-65535).
5. **Authentication Token**: Ensures that if a `token` is provided, it is not blank.
6. **TLS Configuration**: Validates that both `tlsCert` and `tlsKey` are provided together if secure connections are configured.
7. **Binary Path**: Verifies that the `soroban-debug` executable exists at the specified `binaryPath` or can be found in the system `PATH`.

## How to Run Preflight

By default, preflight checks run automatically when you start a debugging session (configurable via `soroban-debugger.preflight.enabled`). 

You can also run the preflight check manually at any time:

1. Open the Command Palette (`Ctrl+Shift+P` or `Cmd+Shift+P` on macOS).
2. Type and select **Soroban: Run Launch Preflight Check**.
3. If you have multiple Soroban launch configurations, you will be prompted to select one.

## Quick Fixes

When preflight detects an issue, it doesn't just show an error—it offers **Quick Fixes** to help you resolve the problem immediately:

- **Missing Contract**: Opens a file picker to locate your compiled `.wasm` contract, automatically updating your `launch.json`.
- **Missing Snapshot**: Prompts you to select a valid state snapshot JSON file.
- **Invalid Configuration**: Opens `launch.json` and highlights the problematic field (e.g., malformed arguments or invalid ports).
- **Missing Launch Config**: If no Soroban configurations exist, offers to generate a default one for you.