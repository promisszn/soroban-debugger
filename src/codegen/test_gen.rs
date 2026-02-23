use crate::runtime::executor::ExecutionRecord;
use crate::{DebuggerError, Result};
use soroban_env_host::xdr::{Limits, WriteXdr};
use std::fs;
use std::path::Path;

/// Template engine for generating Soroban unit tests.
pub struct TestGenerator;

impl TestGenerator {
    /// Generate a Rust unit test from an execution record.
    pub fn generate(record: &ExecutionRecord, wasm_path: &Path) -> Result<String> {
        let mut code = String::new();

        // Imports
        code.push_str("use soroban_sdk::{Env, Symbol, Val, Vec, xdr::ScVal, TryFromVal};\n\n");

        // Test signature
        code.push_str("#[test]\n");
        code.push_str(&format!("fn test_{}_reproduction() {{\n", record.function));

        // Environment setup
        code.push_str("    let env = Env::default();\n");

        // Contract registration
        // We assume the WASM path is relevant to where the test is run.
        // For simplicity, we use the filename and assume it's in the same directory or project root.
        let wasm_file_name = wasm_path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "contract.wasm".to_string());

        code.push_str(&format!(
            "    // Register contract (assuming {} is in current or parent directory)\n",
            wasm_file_name
        ));
        code.push_str(
            "    // You may need to adjust this path depending on your project structure.\n",
        );
        code.push_str(&format!(
            "    let wasm = include_bytes!(\"{}\");\n",
            wasm_file_name
        ));
        code.push_str("    let contract_id = env.register_contract_wasm(None, wasm);\n\n");

        // Prepare arguments
        code.push_str("    // Prepare arguments\n");
        code.push_str("    let mut args = Vec::<Val>::new(&env);\n");
        for arg in &record.args {
            let base64 = arg.to_xdr_base64(Limits::none()).map_err(|e| {
                DebuggerError::ExecutionError(format!("Failed to encode argument to XDR: {:?}", e))
            })?;
            code.push_str(&format!(
                "    args.push_back(Val::try_from_val(&env, &ScVal::from_xdr_base64(\"{}\").unwrap()).unwrap());\n",
                base64
            ));
        }
        code.push('\n');

        // Invocation
        code.push_str(&format!("    // Invoke {}\n", record.function));
        code.push_str(&format!(
            "    let result = env.invoke_contract::<Val>(&contract_id, &Symbol::new(&env, \"{}\"), args);\n\n",
            record.function
        ));

        // Asset result
        code.push_str("    // Assert result\n");
        match &record.result {
            Ok(val) => {
                let base64 = val.to_xdr_base64(Limits::none()).map_err(|e| {
                    DebuggerError::ExecutionError(format!(
                        "Failed to encode result to XDR: {:?}",
                        e
                    ))
                })?;
                code.push_str(&format!(
                    "    let expected = Val::try_from_val(&env, &ScVal::from_xdr_base64(\"{}\").unwrap()).unwrap();\n",
                    base64
                ));
                code.push_str("    assert_eq!(result, expected);\n");
            }
            Err(e) => {
                code.push_str(&format!("    // Note: Execution failed with: {}\n", e));
                code.push_str("    // Add appropriate error assertion here if needed.\n");
            }
        }

        // Storage assertions (optional/partial)
        if !record.storage_after.is_empty() {
            code.push_str("\n    // Storage state assertions (informational)\n");
            for (key, val) in &record.storage_after {
                code.push_str(&format!("    // Key: {}, Value: {}\n", key, val));
            }
        }

        code.push_str("}\n");
        Ok(code)
    }

    /// Write the generated test to a file, either overwriting or appending.
    pub fn write_to_file(path: &Path, content: &str, overwrite: bool) -> Result<()> {
        if path.exists() && !overwrite {
            let mut existing =
                fs::read_to_string(path).map_err(|e| DebuggerError::FileError(e.to_string()))?;
            if !existing.trim().is_empty() {
                existing.push_str("\n\n");
            }
            existing.push_str(content);
            fs::write(path, existing).map_err(|e| DebuggerError::FileError(e.to_string()))?;
        } else {
            // Create parent directories if they don't exist
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| DebuggerError::FileError(e.to_string()))?;
            }
            fs::write(path, content).map_err(|e| DebuggerError::FileError(e.to_string()))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_env_host::xdr::ScVal;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_generate_code() {
        let record = ExecutionRecord {
            function: "add".to_string(),
            args: vec![ScVal::U32(10), ScVal::U32(20)],
            result: Ok(ScVal::U32(30)),
            storage_before: HashMap::new(),
            storage_after: HashMap::new(),
        };
        let wasm_path = Path::new("test.wasm");
        let code = TestGenerator::generate(&record, wasm_path).unwrap();

        assert!(code.contains("fn test_add_reproduction()"));
        assert!(code.contains("let wasm = include_bytes!(\"test.wasm\");"));
        assert!(code.contains("invoke_contract")); // Check for invocation
        assert!(code.contains("assert_eq!(result, expected)"));
    }

    #[test]
    fn test_write_overwrite() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_gen.rs");

        TestGenerator::write_to_file(&file_path, "test1", true).unwrap();
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "test1");

        TestGenerator::write_to_file(&file_path, "test2", true).unwrap();
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "test2");
    }

    #[test]
    fn test_write_append() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_gen.rs");

        TestGenerator::write_to_file(&file_path, "test1", false).unwrap();
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "test1");

        TestGenerator::write_to_file(&file_path, "test2", false).unwrap();
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("test1"));
        assert!(content.contains("\n\ntest2"));
    }
}
