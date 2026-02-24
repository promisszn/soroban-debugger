# Simple Token Contract Example

A minimal token implementation demonstrating core token functionality: minting, transferring, burning, and balance tracking. This contract is designed to showcase the Soroban Debugger's ability to trace token operations and storage changes.

## What this contract does

| Function | Description |
|---|---|
| `initialize(admin, name, symbol)` | Set up the token with metadata and admin |
| `mint(to, amount)` | Create new tokens (admin only) |
| `transfer(from, to, amount)` | Transfer tokens between addresses |
| `burn(from, amount)` | Destroy tokens from an address |
| `balance(account)` | Query token balance for an address |
| `total_supply()` | Query total token supply |
| `name()` | Get token name |
| `symbol()` | Get token symbol |
| `admin()` | Get admin address |

## Build

```bash
cd examples/contracts/simple-token
cargo build --target wasm32-unknown-unknown --release
```

The WASM output will be at:
`target/wasm32-unknown-unknown/release/soroban_simple_token.wasm`

## Debugger Walkthrough

### 1. Initialize the token — watch metadata storage

```bash
soroban-debug inspect \
  --wasm target/wasm32-unknown-unknown/release/soroban_simple_token.wasm \
  --function initialize \
  --args '["GADMIN123", "MyToken", "MTK"]'
```

Expected storage diff:
```
+ Admin        →  GADMIN123
+ Name         →  "MyToken"
+ Symbol       →  "MTK"
+ TotalSupply  →  0
```

### 2. Mint tokens — watch balance and supply increase

```bash
soroban-debug inspect \
  --wasm target/wasm32-unknown-unknown/release/soroban_simple_token.wasm \
  --function mint \
  --args '["GUSER456", 1000000]'
```

Expected storage diff:
```
+ Balance(GUSER456)  →  1000000
~ TotalSupply        :  0  →  1000000
```

### 3. Transfer tokens — watch balances update

```bash
soroban-debug inspect \
  --wasm target/wasm32-unknown-unknown/release/soroban_simple_token.wasm \
  --function transfer \
  --args '["GUSER456", "GRECIPIENT789", 250000]'
```

Expected storage diff:
```
~ Balance(GUSER456)       :  1000000  →  750000
+ Balance(GRECIPIENT789)  →  250000
```

### 4. Burn tokens — watch supply decrease

```bash
soroban-debug inspect \
  --wasm target/wasm32-unknown-unknown/release/soroban_simple_token.wasm \
  --function burn \
  --args '["GUSER456", 100000]'
```

Expected storage diff:
```
~ Balance(GUSER456)  :  750000  →  650000
~ TotalSupply        :  1000000  →  900000
```

### 5. Query balance — read-only operation

```bash
soroban-debug inspect \
  --wasm target/wasm32-unknown-unknown/release/soroban_simple_token.wasm \
  --function balance \
  --args '["GUSER456"]'
```

Expected output:
```
Return value: 650000
Storage diff: (none - read-only)
```

### 6. Step through transfer execution

```bash
soroban-debug step \
  --wasm target/wasm32-unknown-unknown/release/soroban_simple_token.wasm \
  --function transfer \
  --args '["GUSER456", "GRECIPIENT789", 50000]' \
  --breakpoint transfer
```

This allows you to step through the transfer logic instruction by instruction, watching:
- Authorization checks
- Balance reads
- Balance validation
- Balance updates
- Event emission

### 7. Interactive debugging session

```bash
soroban-debug interactive \
  --wasm target/wasm32-unknown-unknown/release/soroban_simple_token.wasm
```

Then run commands interactively:
```
> call initialize GADMIN123 "MyToken" "MTK"
> call mint GUSER456 1000000
> call balance GUSER456
> call transfer GUSER456 GRECIPIENT789 250000
> storage
> events
```

### 8. List all available functions

```bash
soroban-debug list-functions \
  --wasm target/wasm32-unknown-unknown/release/soroban_simple_token.wasm
```

Expected output:
```
initialize
mint
transfer
burn
balance
total_supply
name
symbol
admin
```

### 9. Profile gas usage

```bash
soroban-debug profile \
  --wasm target/wasm32-unknown-unknown/release/soroban_simple_token.wasm \
  --function transfer \
  --args '["GUSER456", "GRECIPIENT789", 100000]'
```

This shows CPU instructions, memory usage, and storage operations for the transfer.

### 10. Compare two execution traces

First, create traces for two different operations:

```bash
soroban-debug run \
  --wasm target/wasm32-unknown-unknown/release/soroban_simple_token.wasm \
  --function mint \
  --args '["GUSER456", 1000]' \
  --output-trace mint_trace.json

soroban-debug run \
  --wasm target/wasm32-unknown-unknown/release/soroban_simple_token.wasm \
  --function transfer \
  --args '["GUSER456", "GRECIPIENT789", 500]' \
  --output-trace transfer_trace.json
```

Then compare them:

```bash
soroban-debug compare mint_trace.json transfer_trace.json
```

This highlights differences in execution paths, storage operations, and gas costs.

## Common Debugging Scenarios

### Scenario 1: Insufficient Balance Error

```bash
soroban-debug inspect \
  --wasm target/wasm32-unknown-unknown/release/soroban_simple_token.wasm \
  --function transfer \
  --args '["GUSER456", "GRECIPIENT789", 999999999]'
```

Expected: Error with code `InsufficientBalance (1)`

### Scenario 2: Zero Amount Error

```bash
soroban-debug inspect \
  --wasm target/wasm32-unknown-unknown/release/soroban_simple_token.wasm \
  --function mint \
  --args '["GUSER456", 0]'
```

Expected: Error with code `ZeroAmount (2)`

### Scenario 3: Unauthorized Mint

```bash
soroban-debug inspect \
  --wasm target/wasm32-unknown-unknown/release/soroban_simple_token.wasm \
  --function mint \
  --args '["GUSER456", 1000]' \
  --caller "GNOT_ADMIN"
```

Expected: Error with code `Unauthorized (3)`

## Storage Layout

The contract uses the following storage keys:

- `Balance(Address)` — Persistent storage for each account's balance
- `TotalSupply` — Instance storage for total token supply
- `Name` — Instance storage for token name
- `Symbol` — Instance storage for token symbol
- `Admin` — Instance storage for admin address

## Events

The contract emits the following events:

- `init` — When token is initialized
- `mint` — When tokens are minted
- `transfer` — When tokens are transferred
- `burn` — When tokens are burned

## Running Tests

```bash
cargo test
```

All tests include comprehensive coverage of:
- Initialization
- Minting (single and multiple)
- Transfers (partial and full)
- Burning (partial and full)
- Error conditions (zero amounts, insufficient balance)
- Multi-user scenarios

## Design Notes

This is a simplified token contract for debugging demonstration purposes. A production token contract would include:
- Allowances and `transfer_from` functionality
- Decimal precision handling
- Access control beyond a single admin
- Pausability
- More sophisticated authorization patterns
- Integration with Soroban's standard token interface (SEP-41)

The simplicity here makes it ideal for learning the debugger's capabilities without getting lost in complex token logic.
