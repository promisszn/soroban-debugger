# Scenario Cookbook

This cookbook provides reusable patterns and "recipes" for common Soroban contract testing scenarios using the `scenario` command and TOML files.

---

## 🏗️ Basic Setup & Initialization

Most scenarios start with initializing a contract or setting up an admin.

```toml
[[steps]]
name = "Initialize Contract"
function = "initialize"
args = '["GD5DJ3B6A2KHSXLYJZ3IGR7Q5UMVJ5J4GQTKTQYQDQXJQJ5YQZQKQZQ", "My Token", "MTK"]'
expected_return = "()"
```

---

## 🔄 Capturing & Reusing Variables

Use the `capture` field to save a return value and `{{var_name}}` to use it in later steps. This is essential for testing dynamically generated values like IDs or total supplies.

```toml
[[steps]]
name = "Mint Tokens"
function = "mint"
args = '["Alice", 1000]'
capture = "new_balance"

[[steps]]
name = "Verify Balance"
function = "get_balance"
args = '["Alice"]'
# Reuses the captured value from the previous step
expected_return = "{{new_balance}}"

[[steps]]
name = "Transfer Captured Amount"
function = "transfer"
args = '["Alice", "Bob", {{new_balance}}]'
```

---

## 💾 Storage Assertions

Verify that internal contract state is updated correctly.

```toml
[[steps]]
name = "Set User Role"
function = "set_role"
args = '["Alice", 1]' # 1 for Admin

[steps.expected_storage]
# Key matches the contract's storage key format
"Role:Alice" = "1"
"TotalAdmins" = "1"
```

---

## 📢 Expected Events

Assert that specific events are emitted during execution.

```toml
[[steps]]
name = "Transfer with Event"
function = "transfer"
args = '["Alice", "Bob", 100]'

# The events must match exactly in order
expected_events = [
    { topics = ["transfer"], data = '["Alice", "Bob", 100]' }
]
```

---

## ⏱️ Timeout Overrides

For heavy steps (e.g., complex loops or large storage migrations), override the default timeout.

```toml
[defaults]
timeout_secs = 10

[[steps]]
name = "Lightweight Step"
function = "ping"

[[steps]]
name = "Heavy Migration"
function = "migrate_all_users"
# Disable timeout for this specific step
timeout_secs = 0 
```

---

## 🛑 Testing Failures (Panics & Errors)

Verify that your contract fails as expected under certain conditions.

```toml
[[steps]]
name = "Transfer Insufficient Funds"
function = "transfer"
args = '["Alice", "Bob", 1000000]'
# Assert the execution fails with a specific error message
expected_error = "InsufficientBalance"

[[steps]]
name = "Invalid Admin Call"
function = "admin_only"
args = '["NonAdmin"]'
# Assert the contract panics
expected_panic = "not authorized"
```

---

## 📊 Budget Constraints

Ensure your contract stays within resource limits.

```toml
[[steps]]
name = "Efficient Operation"
function = "add"
args = "[1, 2]"

[steps.budget_limits]
max_cpu_instructions = 10000
max_memory_bytes = 1024
```

---

## 🚀 End-to-End Walkthrough

This section goes beyond isolated snippets to show the full workflow: authoring a scenario file, running it, and reviewing the execution trace to understand what happened.

We'll use the `simple-token` example contract from `examples/contracts/simple-token/` — a straightforward fungible token with `initialize`, `mint`, `transfer`, and `balance` functions.

### Step 1: Author the Scenario TOML

Create a file named `token_lifecycle.toml` in your working directory:

```toml
# token_lifecycle.toml
# End-to-end test for the simple-token contract.
# Covers: initialization, minting, transfer, balance verification, and an
# expected-failure case for overdrawing.

[defaults]
timeout_secs = 30

# ── Setup ────────────────────────────────────────────────────────────────────

[[steps]]
name = "Initialize Token"
function = "initialize"
args = '["GD5DJ3B6A2KHSXLYJZ3IGR7Q5UMVJ5J4GQTKTQYQDQXJQJ5YQZQKQZQ", "My Token", "MTK"]'
expected_return = "()"

# ── Funding ──────────────────────────────────────────────────────────────────

[[steps]]
name = "Mint 1 000 tokens to Alice"
function = "mint"
args = '["GD726E62Z6XU6KD5J2EPOHG5NQZ5K5I5J5QZQZQZQZQZQZQZQZQZQZQ", 1000]'
expected_return = "()"

[[steps]]
name = "Confirm Alice balance after mint"
function = "balance"
args = '["GD726E62Z6XU6KD5J2EPOHG5NQZ5K5I5J5QZQZQZQZQZQZQZQZQZQZQ"]'
expected_return = "1000"

# ── Transfer ─────────────────────────────────────────────────────────────────

[[steps]]
name = "Alice transfers 300 tokens to Bob"
function = "transfer"
args = '["GD726E62Z6XU6KD5J2EPOHG5NQZ5K5I5J5QZQZQZQZQZQZQZQZQZQZQ", "GD826E62Z6XU6KD5J2EPOHG5NQZ5K5I5J5QZQZQZQZQZQZQZQZQZQZQ", 300]'
expected_return = "()"

# ── State verification ───────────────────────────────────────────────────────

[[steps]]
name = "Verify Alice's remaining balance"
function = "balance"
args = '["GD726E62Z6XU6KD5J2EPOHG5NQZ5K5I5J5QZQZQZQZQZQZQZQZQZQZQ"]'
expected_return = "700"

[steps.expected_storage]
"TotalSupply" = "1000"

[[steps]]
name = "Verify Bob's balance"
function = "balance"
args = '["GD826E62Z6XU6KD5J2EPOHG5NQZ5K5I5J5QZQZQZQZQZQZQZQZQZQZQ"]'
expected_return = "300"

# ── Failure guard ────────────────────────────────────────────────────────────

[[steps]]
name = "Overdraw attempt must fail"
function = "transfer"
args = '["GD726E62Z6XU6KD5J2EPOHG5NQZ5K5I5J5QZQZQZQZQZQZQZQZQZQZQ", "GD826E62Z6XU6KD5J2EPOHG5NQZ5K5I5J5QZQZQZQZQZQZQZQZQZQZQ", 9999]'
expected_error = "insufficient"
```

Key authoring decisions made here:

- **`[defaults]`** sets a 30-second timeout for all steps; individual steps can override it.
- Steps are grouped with comments into setup, funding, transfer, verification, and failure phases — this makes failures easier to locate.
- `expected_storage` on the verification step pins contract-level state, not just the return value.
- The final step uses `expected_error` to assert that the contract correctly rejects an overdraw; the runner treats a matching error as a pass.

### Step 2: Build the Contract

```bash
cd examples/contracts/simple-token
cargo build --target wasm32-unknown-unknown --release
```

The compiled WASM lands at:

```
target/wasm32-unknown-unknown/release/simple_token.wasm
```

### Step 3: Run the Scenario

From the repository root:

```bash
soroban-debugger scenario \
  --contract examples/contracts/simple-token/target/wasm32-unknown-unknown/release/simple_token.wasm \
  --scenario token_lifecycle.toml
```

Add `--verbose` to see per-instruction budget details on each step.

### Step 4: Read the Execution Trace

A successful run prints a step-by-step trace to stdout:

```
ℹ️  Loading scenario file: "token_lifecycle.toml"
ℹ️  Loading contract: "simple_token.wasm"
✅  Running 7 scenario steps...

ℹ️  Step 1: Initialize Token
    Result: ()
    ✅ Return value assertion passed
✅  Step 1 passed.

ℹ️  Step 2: Mint 1 000 tokens to Alice
    Result: ()
    ✅ Return value assertion passed
✅  Step 2 passed.

ℹ️  Step 3: Confirm Alice balance after mint
    Result: 1000
    ✅ Return value assertion passed
✅  Step 3 passed.

ℹ️  Step 4: Alice transfers 300 tokens to Bob
    Result: ()
    ✅ Return value assertion passed
✅  Step 4 passed.

ℹ️  Step 5: Verify Alice's remaining balance
    Result: 700
    ✅ Return value assertion passed
    ✅ Storage assertion passed for key 'TotalSupply'
✅  Step 5 passed.

ℹ️  Step 6: Verify Bob's balance
    Result: 300
    ✅ Return value assertion passed
✅  Step 6 passed.

ℹ️  Step 7: Overdraw attempt must fail
    Error: "insufficient balance"
    ✅ Error assertion matched 'insufficient'
✅  Step 7 passed.

✅  All 7 scenario steps passed successfully!
```

**How to read the trace:**

| Line pattern | Meaning |
|---|---|
| `ℹ️  Step N: <name>` | A new step is starting |
| `    Result: <value>` | The raw return value from the contract |
| `✅ Return value assertion passed` | `expected_return` matched |
| `✅ Storage assertion passed for key '<k>'` | That `expected_storage` key matched |
| `✅ Error assertion matched '<fragment>'` | The actual error contains the expected substring |
| `✅  Step N passed.` | All assertions on this step passed |
| `✅  All N scenario steps passed successfully!` | The whole scenario is green |

### Step 5: Diagnose a Failure

Suppose step 5 fails because a bug causes the transfer to deduct tokens twice:

```
ℹ️  Step 5: Verify Alice's remaining balance
    Result: 400
    ❌ Return value assertion failed! Expected '700', got '400'
⚠️  Step 5 failed.
```

**Triage workflow:**

1. **Isolate the step** — comment out steps 6 and 7 in the TOML, re-run to confirm the failure is reproducible in isolation.
2. **Add a storage assertion** — add `expected_storage` to step 4 to verify what the contract wrote after the transfer:

   ```toml
   [[steps]]
   name = "Alice transfers 300 tokens to Bob"
   function = "transfer"
   args = '["GD726...", "GD826...", 300]'
   expected_return = "()"

   [steps.expected_storage]
   "Balance:GD726E62Z6XU6KD5J2EPOHG5NQZ5K5I5J5QZQZQZQZQZQZQZQZQZQZQ" = "700"
   ```

3. **Inspect with the debugger** — switch from `scenario` to `interactive` to step through the transfer call:

   ```bash
   soroban-debugger interactive \
     --contract simple_token.wasm \
     --function transfer \
     --args '["GD726...", "GD826...", 300]'
   ```

4. **Compare traces** — use `soroban-debugger compare` if you have a known-good trace to diff against the failing one.

Once the bug is fixed, re-run the full scenario to confirm all 7 steps pass again.

### Next Steps

- Add more failure guards using `expected_panic` for contracts that use `panic!` instead of returning errors.
- Extract shared setup steps into an `include`d file (see [Scenario Runner Tutorial](tutorials/scenario-runner.md)).
- Combine this scenario with the symbolic analyzer to auto-generate edge-case steps (see the [Scenario Runner Tutorial § Symbolic Analysis](tutorials/scenario-runner.md#symbolic-analysis)).
