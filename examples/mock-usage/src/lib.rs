#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Symbol};

#[contract]
pub struct MockableContract;

#[contractimpl]
impl MockableContract {
    pub fn hello(env: Env, _name: Symbol) -> Symbol {
        let prefix: Symbol = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "prefix"))
            .unwrap_or(Symbol::new(&env, "Hello"));
        // This is a dummy example to show storage usage
        prefix
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;
    use soroban_debug_mock::MockEnvBuilder;

    #[test]
    fn test_hello_with_mock_storage() {
        let env = Env::default();
        let contract_id = env.register(MockableContract, ());
        let client = MockableContractClient::new(&env, &contract_id);

        let storage_json = json!({
            "prefix": "Hi"
        });

        // Pre-populate storage for the registered contract using the helper
        MockEnvBuilder::from_env(env.clone()).with_contract_storage(&contract_id, &storage_json);

        let result = client.hello(&Symbol::new(&env, "Dev"));
        assert_eq!(result, Symbol::new(&env, "Hi"));
    }
}
