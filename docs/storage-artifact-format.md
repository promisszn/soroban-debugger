# Normalized Storage Artifact Format

Soroban Debugger storage artifacts are used to import and export contract state snapshots across various commands, such as symbolic analysis, replay runs, and interactive debugging.

To ensure artifacts are stable, comparable, and portable, the debugger normalizes exported storage state as follows:

## Deterministic Ordering
All storage entries are sorted alphabetically by their keys during JSON serialization (`BTreeMap`). This prevents arbitrary diffs caused by non-deterministic hash map iteration order.

## JSON Schema
Storage artifacts are serialized as JSON with the following structure:

```json
{
  "schema_version": "1.0.0",
  "entries": {
    "key1": "value1",
    "key2": "value2"
  }
}
```

- `schema_version`: String indicating the format version (e.g., `"1.0.0"`). Used to trigger structural migrations if fields change.
- `entries`: A map of key-value string pairs representing the ledger entries.