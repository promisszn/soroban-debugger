# Escrow Contract Example

A time-locked escrow contract that demonstrates conditional state transitions perfect for debugging with the Soroban Debugger.

## Overview

This escrow contract allows a depositor to lock funds that can be released to a beneficiary after a specified unlock time. The contract showcases three distinct states:
- **Pending**: Funds deposited, awaiting unlock time
- **Released**: Funds released to beneficiary (after unlock time)
- **Refunded**: Funds returned to depositor (before unlock time)

## Contract Functions

### `deposit`
Locks funds in escrow with a time restriction.
```rust
pub fn deposit(
    env: Env,
    depositor: Address,
    beneficiary: Address,
    amount: i128,
    unlock_time: u64,
) -> Result<(), EscrowError>
```

### `release`
Releases funds to beneficiary after unlock time has passed.
```rust
pub fn release(env: Env) -> Result<(), EscrowError>
```

### `refund`
Returns funds to depositor (can be called anytime while status is Pending).
```rust
pub fn refund(env: Env) -> Result<(), EscrowError>
```

### `get_status`
Returns current escrow state information.
```rust
pub fn get_status(env: Env) -> Result<(Address, Address, i128, u64, EscrowStatus), EscrowError>
```

## Building the Contract

```bash
cd examples/contracts/escrow
cargo build --target wasm32-unknown-unknown --release
```

The compiled WASM file will be at:
```
target/wasm32-unknown-unknown/release/soroban_escrow.wasm
```

## Debugging Walkthrough

This section demonstrates how to use the Soroban Debugger to observe state transitions in the escrow contract.

### Step 1: Deposit Funds into Escrow

First, let's deposit 1000 units into escrow with an unlock time of 100:

```bash
soroban-debug run \
  --contract target/wasm32-unknown-unknown/release/soroban_escrow.wasm \
  --function deposit \
  --args '[
    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFGHL",
    "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
    1000,
    100
  ]'
```

**Expected Output:**
```
✓ Execution completed successfully

Storage Changes:
  Depositor    → GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFGHL
  Beneficiary  → GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB
  Amount       → 1000
  UnlockTime   → 100
  Status       → Pending (0)

Events:
  deposit(depositor, beneficiary, 1000, 100)

Budget:
  CPU: 1,245 instructions
  Memory: 512 bytes
```

**What to observe:**
- All five storage keys are created (Depositor, Beneficiary, Amount, UnlockTime, Status)
- Status is set to `Pending` (represented as 0)
- A `deposit` event is emitted

### Step 2: Check Escrow Status

Query the current state:

```bash
soroban-debug run \
  --contract target/wasm32-unknown-unknown/release/soroban_escrow.wasm \
  --function get_status
```

**Expected Output:**
```
✓ Execution completed successfully

Return Value: (
  GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFGHL,
  GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB,
  1000,
  100,
  Pending
)

Storage (Read-only):
  Depositor    → GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFGHL
  Beneficiary  → GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB
  Amount       → 1000
  UnlockTime   → 100
  Status       → Pending (0)
```

### Step 3: Attempt Early Release (Should Fail)

Try to release funds before the unlock time:

```bash
soroban-debug run \
  --contract target/wasm32-unknown-unknown/release/soroban_escrow.wasm \
  --function release
```

**Expected Output:**
```
✗ Execution failed

Error: TooEarly (4)
  Cannot release before unlock time

Current State:
  Status       → Pending (0)
  UnlockTime   → 100
  Current Time → 0

Debug tip: Set ledger timestamp >= unlock_time to allow release
```

**What to observe:**
- The contract correctly rejects early release attempts
- Error code 4 corresponds to `EscrowError::TooEarly`
- Storage remains unchanged (Status still Pending)

### Step 4: Release After Unlock Time

To simulate time passing, we need to mock the ledger timestamp. For this, you would typically use the test environment or a snapshot with the appropriate ledger state.

In a real debugging scenario with a snapshot file:

```bash
soroban-debug run \
  --contract target/wasm32-unknown-unknown/release/soroban_escrow.wasm \
  --function release \
  --storage '{"ledger": {"timestamp": 150}}'
```

**Expected Output:**
```
✓ Execution completed successfully

Storage Changes:
  Status: Pending (0) → Released (1)

Events:
  release(GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB, 1000)

Budget:
  CPU: 987 instructions
  Memory: 340 bytes
```

**What to observe:**
- **State transition**: Status changes from `Pending` (0) to `Released` (1)
- Only the Status field changes; other fields remain intact
- A `release` event is emitted with beneficiary and amount

### Step 5: Refund Scenario

To demonstrate the refund path, start fresh with a new deposit, then refund:

```bash
# Deposit
soroban-debug run \
  --contract target/wasm32-unknown-unknown/release/soroban_escrow.wasm \
  --function deposit \
  --args '[
    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFGHL",
    "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
    500,
    200
  ]'

# Refund
soroban-debug run \
  --contract target/wasm32-unknown-unknown/release/soroban_escrow.wasm \
  --function refund
```

**Expected Output (Refund):**
```
✓ Execution completed successfully

Storage Changes:
  Status: Pending (0) → Refunded (2)

Events:
  refund(GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAFGHL, 500)

Budget:
  CPU: 945 instructions
  Memory: 335 bytes
```

**What to observe:**
- **State transition**: Status changes from `Pending` (0) to `Refunded` (2)
- Different event emitted (`refund` instead of `release`)
- Depositor receives the refund

### Step 6: Inspect State Transitions with Storage Diff

Use the debugger's storage filtering to track specific state changes:

```bash
soroban-debug run \
  --contract target/wasm32-unknown-unknown/release/soroban_escrow.wasm \
  --function release \
  --storage-filter 'Status'
```

**Expected Output:**
```
Storage Changes (filtered):
  Status: Pending → Released

All other storage keys unchanged (filtered out)
```

## Debugging Tips

### 1. Track State Transitions
Use `--storage-filter 'Status'` to focus on status changes:
```bash
--storage-filter 'Status'
```

### 2. Compare Before/After States
Export execution traces to compare state changes:
```bash
soroban-debug run \
  --contract contract.wasm \
  --function deposit \
  --trace-output deposit_trace.json

soroban-debug run \
  --contract contract.wasm \
  --function release \
  --trace-output release_trace.json

soroban-debug compare deposit_trace.json release_trace.json
```

### 3. Test Error Conditions
Deliberately trigger errors to see error handling:
- Try depositing zero amount
- Try releasing before unlock time
- Try refunding after release

### 4. Interactive Debugging
Use interactive mode to step through state changes:
```bash
soroban-debug interactive \
  --contract target/wasm32-unknown-unknown/release/soroban_escrow.wasm

> break deposit
> break release
> run deposit ADDR1 ADDR2 1000 100
> storage
> continue
```

## Common Issues

**Q: Why does `release` fail with "TooEarly"?**
A: The current ledger timestamp is before the `unlock_time`. In tests, use `env.ledger().with_mut(|li| li.timestamp = ...)` to set the time.

**Q: Can I deposit multiple times?**
A: No, the contract allows only one escrow per instance. Trying to deposit again returns `AlreadyInitialized` error.

**Q: What happens if I try to release after refund?**
A: You'll get `AlreadyFinalized` error because the status is no longer `Pending`.

## Learning Objectives

This example teaches you to:
- ✅ Debug conditional state machines
- ✅ Observe storage state transitions (Pending → Released/Refunded)
- ✅ Track time-based logic in contracts
- ✅ Inspect error conditions and edge cases
- ✅ Use storage filtering for focused debugging
