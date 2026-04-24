# Snapshot Compression

The Soroban Debugger allows exporting the final storage state (snapshot) to a file after execution using the `--export-storage` flag. Depending on the contract and network state size, these snapshots can become quite large.

To help manage storage and transfer costs, you can export these snapshots in compressed formats by using the `--export-compression` flag.

## Usage

You can specify the compression algorithm using the `--export-compression` argument. Supported formats are:

- `none` (default): Exports as plain text JSON.
- `gzip`: Exports the JSON snapshot compressed with Gzip (`.gz` extension recommended).
- `zstd`: Exports the JSON snapshot compressed with Zstandard (`.zst` extension recommended).

### Example: Exporting with Gzip

```bash
soroban-debug run \
  --contract my_contract.wasm \
  --function init \
  --export-storage state.json.gz \
  --export-compression gzip
```

### Example: Exporting with Zstandard

```bash
soroban-debug run \
  --contract my_contract.wasm \
  --function init \
  --export-storage state.json.zst \
  --export-compression zstd
```

## Replay Artifacts Metadata

When generating replay artifacts, the `compression` metadata field is explicitly populated in the sidecar manifest to ensure the debugger knows how to decompress the artifact when it is imported back into a session.

By explicitly setting the compression type, you maintain predictable file naming conventions and allow the toolchain to handle decompression transparently.