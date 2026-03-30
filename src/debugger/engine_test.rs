#[cfg(test)]
mod tests {
    use super::*;
    use soroban_env_host::xdr::{ContractEvent, ContractEventBody, ScVal, ScSymbol};

    fn create_test_engine() -> DebuggerEngine {
        // Create a minimal WASM for testing - use empty bytes since we're not executing
        let wasm_bytes = vec![
            0x00, 0x61, 0x73, 0x6d, // WASM magic
            0x01, 0x00, 0x00, 0x00, // WASM version
        ];
        let executor = crate::runtime::executor::ContractExecutor::new(wasm_bytes).unwrap();
        DebuggerEngine::new(executor, vec![])
    }

    #[test]
    fn test_get_first_event_topic_with_fn_call() {
        let engine = create_test_engine();

        // Create a diagnostic event with fn_call topic
        let fn_call_symbol = ScSymbol(b"fn_call".to_vec());
        let event = ContractEvent {
            contract_id: None,
            body: ContractEventBody::V0(soroban_env_host::xdr::ContractEventV0 {
                topics: vec![ScVal::Symbol(fn_call_symbol)],
                data: ScVal::Void,
            }),
        };

        let topic = engine.get_first_event_topic(&event);
        assert_eq!(topic, Some("fn_call".to_string()));
    }

    #[test]
    fn test_get_first_event_topic_with_fn_return() {
        let engine = create_test_engine();

        // Create a diagnostic event with fn_return topic
        let fn_return_symbol = ScSymbol(b"fn_return".to_vec());
        let event = ContractEvent {
            contract_id: None,
            body: ContractEventBody::V0(soroban_env_host::xdr::ContractEventV0 {
                topics: vec![ScVal::Symbol(fn_return_symbol)],
                data: ScVal::Void,
            }),
        };

        let topic = engine.get_first_event_topic(&event);
        assert_eq!(topic, Some("fn_return".to_string()));
    }

    #[test]
    fn test_get_first_event_topic_with_empty_topics() {
        let engine = create_test_engine();

        // Create an event with no topics
        let event = ContractEvent {
            contract_id: None,
            body: ContractEventBody::V0(soroban_env_host::xdr::ContractEventV0 {
                topics: vec![],
                data: ScVal::Void,
            }),
        };

        let topic = engine.get_first_event_topic(&event);
        assert_eq!(topic, None);
    }

    #[test]
    fn test_get_first_event_topic_with_non_symbol_topic() {
        let engine = create_test_engine();

        // Create an event with a non-symbol topic
        let event = ContractEvent {
            contract_id: None,
            body: ContractEventBody::V0(soroban_env_host::xdr::ContractEventV0 {
                topics: vec![ScVal::U32(42)],
                data: ScVal::Void,
            }),
        };

        let topic = engine.get_first_event_topic(&event);
        assert_eq!(topic, Some("U32(42)".to_string()));
    }
}
