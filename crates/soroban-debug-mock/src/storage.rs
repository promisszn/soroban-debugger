use serde_json::Value;
use soroban_sdk::{Env, Symbol};

pub struct StorageHelper;

impl StorageHelper {
    pub fn populate_from_json(env: &Env, json: &Value) {
        if let Some(storage) = json.as_object() {
            for (key_str, value) in storage {
                // For simplicity in this helper, we assume keys are symbols
                // and values are JSON that we try to convert.
                // In a full implementation, we'd need more robust conversion logic.
                let key = Symbol::new(env, key_str);

                // Simplified value handling for MVP
                if let Some(i) = value.as_i64() {
                    env.storage().instance().set(&key, &i);
                } else if let Some(b) = value.as_bool() {
                    env.storage().instance().set(&key, &b);
                } else if let Some(s) = value.as_str() {
                    env.storage().instance().set(&key, &Symbol::new(env, s));
                }
            }
        }
    }
}
