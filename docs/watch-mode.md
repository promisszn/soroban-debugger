# Watch Mode - DEPRECATED

> **⚠️ DEPRECATED:** The `--watch` flag is not currently implemented in the CLI. This documentation is retained for reference only.

## Recommended Workflow

Until watch mode is restored, use this workflow for development:

1. In one terminal, compile your contract with file watching:
   ```bash
   cd my_contract && cargo watch -x 'build --target wasm32-unknown-unknown --release'
   ```

2. In another terminal, run the debugger once per change:
   ```bash
   soroban-debug run \
     --contract target/wasm32-unknown-unknown/release/my_contract.wasm \
     --function my_function \
     --args '[...]'
   ```

This achieves the same goal as watch mode while using standard Rust tooling.
