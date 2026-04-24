# Contributor Command Map

When you add or modify a command, flag, or feature in the Soroban Debugger, the change often ripples across multiple surfaces: the CLI, the VS Code extension, documentation, and automated tests.

Use this map to identify all the files you need to touch to keep the repository in sync and prevent accidental omissions.

## 1. Rust CLI & Core Engine

If you are adding a CLI argument or changing the core engine:

- **CLI Arguments**: `src/cli/` (update the `clap` struct definitions).
- **Implementation**: `src/debugger/`, `src/inspector/`, or `src/runtime/` as appropriate.
- **Man Pages**: Run `make regen-man` to update `man/man1/*.1`. (Do not hand-edit these; they are generated automatically from `clap` docs).

## 2. VS Code Extension

If your feature should be accessible from the VS Code editor (e.g., exposed as an option in `launch.json` or `attach` configurations):

- **Extension Manifest**: Add the new property to `extensions/vscode/package.json` under `contributes.debuggers.configurationAttributes.launch.properties` (or `.attach.properties`).
- **Manifest Schema**: Update `extensions/vscode/package.schema.json` to match the new `package.json` field. (This strict draft-07 JSON schema is enforced in CI).
- **Extension Preflight**: If the flag represents an environment dependency or file path, add validation logic in `extensions/vscode/src/preflightCommand.ts`.
- **Adapter Logic**: Read the new configuration field in `extensions/vscode/src/dap/adapter.ts` and pass it to the spawned CLI process in `extensions/vscode/src/cli/debuggerProcess.ts`.

## 3. Documentation

Do not leave documentation stale! Consider which of the following need updates:

- **Feature Matrix**: `docs/feature-matrix.md`. Add a row for the new feature detailing if it is supported in the CLI, Extension, or both.
- **Command Groups**: `docs/cli-command-groups.md`. Categorize your new command.
- **Topic Guides**: Add coverage to the relevant reference doc (e.g., `docs/instruction-stepping.md`, `docs/remote-debugging.md`, `docs/batch-execution.md`).
- **README**: If it's a major feature, update the Quick Start or Features lists in `README.md`.
- **Extension README**: If you added an extension configuration, document it under the `Debug Configuration Options` section in `extensions/vscode/README.md`.

## 4. Tests

Features require automated tests to prevent regressions:

- **CLI Tests**: Add an integration test in the `tests/` directory verifying standard execution, success, and specific failure behavior.
- **Extension Tests**: Add end-to-end DAP coverage in `extensions/vscode/src/test/runDapE2E.ts` if modifying adapter behavior.