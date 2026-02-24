# Soroban Debugger FAQ

This page covers common questions, confusing behaviors, and troubleshooting tips for the Soroban Debugger (`soroban-debug`).

## Categories
- [Installation](#installation)
- [Running Contracts](#running-contracts)
- [Breakpoints](#breakpoints)
- [Budget](#budget)
- [Output and Trace](#output-and-trace)
- [Argument Parsing](#argument-parsing)

---

## Installation

### 1. `cargo install` fails with "linker 'cc' not found"
**Cause:** Your system lacks the necessary build tools (C compiler and linker) required to compile Rust dependencies.
**Fix:**
- **Windows:** Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and select the "Desktop development with C++" workload.
- **Linux:** Install `build-essential` (Ubuntu/Debian) or `base-devel` (Arch).
- **macOS:** Run `xcode-select --install`.

### 2. Can't access `man` pages after installation
**Cause:** `cargo install` only installs the binary, not the manual pages.
**Fix:** If building from source, manually copy the man pages:
```bash
sudo cp man/man1/soroban-debug* /usr/local/share/man/man1/
```
Then use `man soroban-debug`.

### 3. Error: "Rust 1.75 or later required"
**Cause:** The project uses modern Rust features.
**Fix:** Update your Rust toolchain:
```bash
rustup update
```

---

## Running Contracts

### 4. WASM load failure: "No such file or directory"
**Cause:** The path provided to `--contract` is incorrect or relative to a different directory.
**Fix:** Ensure the path is correct relative to your current working directory. Use an absolute path if unsure.
```bash
soroban-debug run --contract ./target/wasm32-unknown-unknown/release/my_contract.wasm ...
```

### 5. "Function not found" error
**Cause:** The function name specified with `--function` is not exported by the WASM contract or is misspelled.
**Fix:** Use the `inspect` command to see all available exported functions:
```bash
soroban-debug inspect --contract my_contract.wasm
```

### 6. Contract panics with "Host error: Unknown error"
**Cause:** This usually happens when an assertion fails or an unexpected state is reached during execution.
**Fix:** Start an interactive session to step through the code and identify the exact instruction causing the panic:
```bash
soroban-debug interactive --contract my_contract.wasm
```

### 7. Watch mode (`--watch`) doesn't reload when I save my Rust code
**Cause:** Watch mode monitors the **WASM file**, not your Rust source files.
**Fix:** You need a separate process (like `cargo watch`) to rebuild your WASM file. Once the WASM file is updated on disk, `soroban-debug` will detect the change and re-run.

---

## Breakpoints

### 8. Breakpoints are not triggering
**Cause:** You might be setting a breakpoint on a function that is never called, or the function name is slightly different (e.g., due to name mangling, though Soroban usually keeps them clean).
**Fix:** Verify the function name using `soroban-debug inspect`. In `interactive` mode, use `list-breaks` to ensure your breakpoints are registered.

### 9. Can I set a breakpoint on a specific line number?
**Answer:** Currently, the debugger supports setting breakpoints only at **function boundaries**.
**Workaround:** Set a breakpoint at the function containing the line, then use `s` (step) or `n` (next) to reach the specific line you're interested in.

---

## Budget

### 10. Why am I getting "Warning: High CPU usage detected"?
**Cause:** The contract has consumed a significant portion of the Soroban CPU budget.
**Fix:** Optimize expensive loops, reduce deep recursion, or minimize complex storage operations. Use the `budget` command in interactive mode to see which parts of your code are the most "expensive".

### 11. "Budget exceeded" error during debugging
**Cause:** The execution hit the maximum allowed Soroban resource limits.
**Fix:** Check for infinite loops or extremely inefficient algorithms. You can also try to provide a larger initial budget if your local environment allows (though on-chain limits will still apply).

### 12. Debugger budget numbers don't match exactly with on-chain execution
**Cause:** The debugger environment might have slight overhead or use a different version of the Soroban host than the network you are targeting.
**Fix:** Use budget numbers as a relative guide for optimization rather than an absolute guarantee for on-chain costs.

---

## Argument Parsing

### 13. My JSON arguments are failing to parse
**Cause:** Shell quoting issues are common. If your JSON contains double quotes, the shell might be stripping them.
**Fix:** Wrap the entire JSON string in single quotes:
```bash
soroban-debug run --args '["Alice", "Bob", 100]'
```

### 14. Error: "Type/value mismatch: expected u32 but got 5000000000"
**Cause:** The value provided exceeds the range of the target type (e.g., `u32` max is ~4.29 billion).
**Fix:** Ensure your input fits within the specified type, or use a larger type like `u64` or `i128` (default).

### 15. How do I pass a Soroban Address as an argument?
**Answer:** Use the explicit type annotation for addresses.
**Fix:**
```json
{"type": "address", "value": "CCV6S6F6..."}
```
Or, if it's a 56-character string starting with 'C' or 'G', the debugger will often auto-detect it as an Address.

---

## Output and Trace

### 16. The trace file is too large and hard to read
**Cause:** Exporting every storage change and event can lead to huge JSON files.
**Fix:** Use `--storage-filter` to only include the keys you care about in the output, which will also reduce the trace size.
```bash
soroban-debug run --trace-output trace.json --storage-filter 'balance:*'
```

### 17. The terminal output looks garbled or has weird characters
**Cause:** Your terminal might not support Unicode box-drawing characters or ANSI colors.
**Fix:** Use the `--no-unicode` flag and set the `NO_COLOR=1` environment variable:
```bash
NO_COLOR=1 soroban-debug run --no-unicode ...
```
