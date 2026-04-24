# Soroban Debug Mock Helpers

The `soroban-debug-mock` crate provides ergonomic helpers to wrap common patterns for repeated storage setup, event expectations, and call choreography, significantly reducing the boilerplate required for testing contract scenarios.

## `MockBuilder`

The `MockBuilder` provides a fluent interface for setting up the execution environment, configuring initial storage, and defining mock cross-contract call responses.

### Example

```rust
use soroban_sdk::{Env, Symbol, IntoVal};
use soroban_debug_mock::builder::MockBuilder;

let env = Env::default();
let builder = MockBuilder::new(&env)
    .with_contract_id("CA7QYNF5GE5XEC4HALXWFVQQ5TQWQ5LF7WMXMEQG7BWHBQV26YCWL5")
    .with_storage(
        Symbol::new(&env, "counter").into_val(&env),
        10u32.into_val(&env),
    )
    .mock_call("other_contract_id", "some_function", 42u32.into_val(&env));

builder.execute_mock();
```

## `StorageAssertions`

Simplifies checking the post-execution state of a contract's storage.

```rust
use soroban_debug_mock::assertions::StorageAssertions;

let asserts = StorageAssertions::new(&env);
asserts.assert_has_key("CA7Q...", Symbol::new(&env, "counter").into_val(&env));
asserts.assert_value_eq("CA7Q...", Symbol::new(&env, "counter").into_val(&env), 10u32.into_val(&env));
```

## `EventAssertions`

Provides tools for verifying the events emitted during the mocked execution.

```rust
use soroban_sdk::vec;
use soroban_debug_mock::assertions::EventAssertions;

let event_asserts = EventAssertions::new(&env);

// Ensure exactly one event was emitted
event_asserts.assert_event_count(1);

// Assert a specific event was emitted
let expected_topics = vec![&env, Symbol::new(&env, "transfer").into_val(&env)];
let expected_data = 100u32.into_val(&env);

event_asserts.assert_event_emitted(expected_topics, expected_data);
```