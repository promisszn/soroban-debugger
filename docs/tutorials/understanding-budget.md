# Understanding Budget and Resource Limits

This tutorial explains Soroban's CPU and memory budget system and how to use the debugger to understand, diagnose, and optimize resource usage in your smart contracts.

## What is Budget in Soroban?

Soroban uses a **resource budgeting system** to ensure that smart contracts execute within predictable resource limits. Every contract execution consumes two types of resources:

1. **CPU Instructions**: The number of computational steps (WASM instructions) executed
2. **Memory Bytes**: The amount of memory allocated during execution

Both resources have **hard limits** on the Stellar network to prevent infinite loops, excessive computation, and resource exhaustion attacks.

### Why Does Budget Matter?

- ✅ **Predictable Costs**: Every operation has a known resource cost
- ✅ **Network Protection**: Prevents malicious contracts from consuming excessive resources
- ✅ **Performance Optimization**: Helps you identify and optimize expensive operations
- ✅ **Transaction Success**: Exceeding budget limits causes transaction failures on-chain

## Reading Budget Output from the Debugger

When you run a contract with the Soroban debugger, budget information is displayed automatically:

```bash
soroban-debug run \
  --contract my_contract.wasm \
  --function my_function \
  --args '[10]'
```

**Example Output:**
```
✓ Execution completed successfully

Budget:
  CPU Instructions: 1,245 / 100,000,000 (0.001%)
  Memory Bytes: 512 / 41,943,040 (0.001%)
```

### Understanding the Budget Output

| Field | Description |
|-------|-------------|
| **CPU Instructions** | `consumed / limit (percentage)` |
| **Memory Bytes** | `consumed / limit (percentage)` |

**What the numbers mean:**
- **Consumed**: Actual resources used by this execution
- **Limit**: Maximum allowed (network limits)
- **Percentage**: How close you are to hitting the limit

## Example: Budget-Heavy Contract

Let's examine a contract that performs expensive operations to see budget consumption in action.

### The Contract

Here's a simple contract that performs repeated vector operations and storage writes:

```rust
#![no_std]
use soroban_sdk::{contract, contractimpl, symbol_short, Env, Vec};

#[contract]
pub struct BudgetHeavy;

#[contractimpl]
impl BudgetHeavy {
    pub fn heavy(env: Env, n: u32) -> u32 {
        let mut v = Vec::<u32>::new(&env);
        for i in 0..n {
            v.push_back(i);
            env.storage().instance().set(&symbol_short!("i"), &i);
        }
        v.len()
    }
}
```

**What makes this expensive:**
- Creates and grows a vector dynamically
- Performs `n` storage writes (very expensive)
- Loops `n` times (scales linearly with input)

### Running with Small Input (n=10)

```bash
soroban-debug run \
  --contract tests/fixtures/contracts/budget_heavy/target/wasm32-unknown-unknown/release/budget_heavy.wasm \
  --function heavy \
  --args '[10]'
```

**Output:**
```
✓ Execution completed successfully

Return Value: 10

Budget:
  CPU Instructions: 15,234 / 100,000,000 (0.015%)
  Memory Bytes: 1,024 / 41,943,040 (0.002%)

Storage Writes: 10
```

**Analysis:**
- With `n=10`, budget usage is minimal (<0.1%)
- Safe and well within limits
- No optimization needed

### Running with Large Input (n=10,000)

```bash
soroban-debug run \
  --contract tests/fixtures/contracts/budget_heavy/target/wasm32-unknown-unknown/release/budget_heavy.wasm \
  --function heavy \
  --args '[10000]'
```

**Output:**
```
✓ Execution completed successfully

Return Value: 10000

Budget:
  CPU Instructions: 8,456,789 / 100,000,000 (8.5%)
  Memory Bytes: 45,678 / 41,943,040 (0.1%)

Storage Writes: 10,000

⚠ Warning: High CPU usage detected (>5%)
```

**Analysis:**
- CPU usage jumped to 8.5% (scaling linearly with input)
- Storage writes increased proportionally
- Debugger warns when usage exceeds 5%
- Still safe, but approaching concerning levels

### Running with Very Large Input (n=1,000,000)

```bash
soroban-debug run \
  --contract tests/fixtures/contracts/budget_heavy/target/wasm32-unknown-unknown/release/budget_heavy.wasm \
  --function heavy \
  --args '[1000000]'
```

**Output:**
```
✗ Execution failed

Error: Budget limit exceeded

Budget:
  CPU Instructions: 100,000,000 / 100,000,000 (100.0%) ⚠ EXCEEDED
  Memory Bytes: 4,234,567 / 41,943,040 (10.1%)

Failure Point: Loop iteration 876,234
Function: heavy
Recommendation: Reduce loop iterations or optimize expensive operations
```

**Analysis:**
- **Budget exceeded**: Contract execution halted
- CPU limit hit at ~876k iterations
- This transaction would **fail on-chain**
- **Action required**: Optimize or reduce input size

## Budget Warning Thresholds

The Soroban debugger provides helpful warnings at different budget usage levels:

| Threshold | Level | Message |
|-----------|-------|---------|
| **> 5%** | ⚠️ Warning | "High resource usage detected" |
| **> 25%** | ⚠️ Caution | "Approaching budget limits" |
| **> 50%** | ⚠️ Critical | "Critical budget usage - optimize immediately" |
| **≥ 100%** | ❌ Error | "Budget limit exceeded" |

These warnings help you catch budget issues **before deployment** rather than discovering them on-chain.

## Common Budget-Expensive Operations

Understanding which operations consume the most resources helps you optimize:

### High CPU Cost
- ✅ **Loops** (especially nested loops)
- ✅ **Complex arithmetic** (division, modulo)
- ✅ **Storage reads/writes** (very expensive)
- ✅ **Vector operations** (push, pop, iteration)
- ✅ **String manipulation**
- ✅ **Cryptographic operations** (hashing, signing)

### High Memory Cost
- ✅ **Large vectors** or maps
- ✅ **String allocation**
- ✅ **Nested data structures**
- ✅ **Multiple contract calls** (stack growth)

## Optimizing Budget Usage

### Strategy 1: Reduce Loop Iterations

**Before:**
```rust
pub fn process_all(env: Env, items: Vec<u32>) {
    for item in items.iter() {
        env.storage().instance().set(&symbol_short!("item"), &item);
    }
}
```

**After:**
```rust
pub fn process_batch(env: Env, items: Vec<u32>, batch_size: u32) {
    for i in 0..batch_size.min(items.len()) {
        let item = items.get(i).unwrap();
        env.storage().instance().set(&symbol_short!("item"), &item);
    }
}
```

**Improvement:** Limits iterations to a safe maximum

### Strategy 2: Batch Storage Writes

**Before:**
```rust
for i in 0..n {
    env.storage().instance().set(&symbol_short!("val"), &i);
}
```

**After:**
```rust
// Store final value once instead of every iteration
let mut result = 0;
for i in 0..n {
    result = i;
}
env.storage().instance().set(&symbol_short!("val"), &result);
```

**Improvement:** Reduces storage writes from `n` to `1`

### Strategy 3: Use Efficient Data Structures

**Before:**
```rust
let mut vec = Vec::<u32>::new(&env);
for i in 0..1000 {
    vec.push_back(i); // Resizes frequently
}
```

**After:**
```rust
let vec = Vec::<u32>::from_array(&env, &[/* pre-sized array */]);
```

**Improvement:** Pre-allocates memory, avoiding repeated resizing

## Debugging Budget Issues: Step-by-Step

### Step 1: Identify the Problem

Run your contract and check budget output:

```bash
soroban-debug run --contract contract.wasm --function expensive_func
```

Look for:
- ⚠️ High percentage (>25%)
- ⚠️ Warning messages
- ❌ Budget exceeded errors

### Step 2: Profile with Breakpoints

Set breakpoints to measure budget at specific points:

```bash
soroban-debug interactive --contract contract.wasm

> break my_expensive_loop
> run expensive_func
> budget
> continue
> budget
```

Compare budget before and after specific operations.

### Step 3: Use Batch Execution for Regression Testing

Create a test suite to track budget across inputs:

```json
[
  {
    "args": "[10]",
    "label": "Small input",
    "expected_budget_max": 20000
  },
  {
    "args": "[100]",
    "label": "Medium input",
    "expected_budget_max": 200000
  }
]
```

```bash
soroban-debug run \
  --contract contract.wasm \
  --function heavy \
  --batch-args budget_tests.json
```

### Step 4: Compare Traces

Export traces before and after optimization:

```bash
# Before optimization
soroban-debug run \
  --contract contract_v1.wasm \
  --function process \
  --trace-output before.json

# After optimization
soroban-debug run \
  --contract contract_v2.wasm \
  --function process \
  --trace-output after.json

# Compare
soroban-debug compare before.json after.json
```

Look for budget improvements in the diff.

## Real-World Budget Limits

**Current Soroban Mainnet Limits (as of 2024):**

| Resource | Limit | Notes |
|----------|-------|-------|
| CPU Instructions | 100,000,000 | Per transaction |
| Memory Bytes | 41,943,040 (~40 MB) | Per transaction |

**What this means in practice:**
- Simple transfers: ~1,000-5,000 instructions
- Token mints: ~10,000-50,000 instructions
- Complex DeFi operations: ~500,000-2,000,000 instructions
- Very heavy computation: >10,000,000 instructions (rare, needs optimization)

## Budget Best Practices

1. ✅ **Test with realistic inputs**: Don't just test with `n=1`
2. ✅ **Set budget targets**: Aim for <10% usage for safety margin
3. ✅ **Profile early**: Check budget during development, not after deployment
4. ✅ **Batch operations**: Group expensive operations when possible
5. ✅ **Avoid unbounded loops**: Always have a maximum iteration count
6. ✅ **Cache storage reads**: Read once, reuse the value
7. ✅ **Monitor regressions**: Use batch testing to catch budget increases

## Additional Resources

- [Soroban Resource Model (Official Docs)](https://soroban.stellar.org/docs/learn/persisting-data#resource-model)
- [Stellar Network Resource Limits](https://developers.stellar.org/docs/learn/smart-contract-internals/resource-limits-fees)
- [Debugger FAQ - Budget Issues](../faq.md#budget)

## Summary

- **Budget = CPU + Memory limits** that prevent resource exhaustion
- **Debugger shows budget usage** with warnings at 5%, 25%, 50%, and 100%
- **Loops and storage operations** are typically the most expensive
- **Optimize by**: reducing iterations, batching storage, using efficient data structures
- **Test early** with realistic inputs to catch budget issues before deployment

With the Soroban debugger, you can identify and fix budget problems **before they cause on-chain failures**, ensuring your contracts are efficient, reliable, and cost-effective.
