use crate::mock::MockRegistry;
use crate::storage::StorageHelper;
use serde_json::Value;
use soroban_sdk::{Address, Env, Val};

pub struct MockEnvBuilder {
    env: Env,
    mock_registry: MockRegistry,
}

impl Default for MockEnvBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MockEnvBuilder {
    pub fn new() -> Self {
        Self::from_env(Env::default())
    }

    pub fn from_env(env: Env) -> Self {
        Self {
            env,
            mock_registry: MockRegistry::default(),
        }
    }

    pub fn with_contract_storage(self, address: &Address, json: &Value) -> Self {
        self.env.as_contract(address, || {
            StorageHelper::populate_from_json(&self.env, json);
        });
        self
    }

    pub fn with_storage_json(self, json: &Value) -> Self {
        // Default to a temporary contract if no address is provided?
        // Or just document that it uses the current context.
        // For now, let's keep it but maybe it's less useful than with_contract_storage.
        StorageHelper::populate_from_json(&self.env, json);
        self
    }

    pub fn with_mock_call(mut self, contract_id: &str, function: &str, return_value: Val) -> Self {
        self.mock_registry
            .register(contract_id, function, return_value);
        self
    }

    pub fn build(self) -> Env {
        self.mock_registry.install(&self.env);
        self.env
    }
}
