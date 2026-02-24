use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorExplanation {
    pub code: u32,
    pub name: String,
    pub description: String,
    pub common_cause: String,
    pub suggested_fix: String,
}

pub struct ErrorDatabase {
    standard_errors: HashMap<u32, ErrorExplanation>,
    custom_errors: HashMap<u32, ErrorExplanation>,
}

impl ErrorDatabase {
    pub fn new() -> Self {
        let mut db = Self {
            standard_errors: HashMap::new(),
            custom_errors: HashMap::new(),
        };
        db.init_standard_errors();
        db
    }

    fn init_standard_errors(&mut self) {
        let std_errs = [
            (
                1,
                "UnknownError",
                "An unknown error occurred during contract execution",
                "Unhandled exception or unexpected state",
                "Check contract logic for unhandled edge cases or add error handling",
            ),
            (
                2,
                "HostError",
                "Error from the Soroban host environment",
                "Host operation failed (e.g., storage, budget exceeded)",
                "Check budget limits, storage access, or host resource constraints",
            ),
            (
                3,
                "ConversionError",
                "Failed to convert between value types",
                "Type mismatch or invalid value format",
                "Verify argument types match function signature and value formats are correct",
            ),
            (
                4,
                "StorageError",
                "Storage operation failed",
                "Storage key not found, access denied, or storage limit exceeded",
                "Check storage key existence, permissions, and storage budget limits",
            ),
            (
                5,
                "BudgetError",
                "CPU or memory budget exceeded",
                "Contract execution consumed too many CPU instructions or memory bytes",
                "Optimize contract code, reduce loop iterations, or use more efficient algorithms",
            ),
            (
                6,
                "AuthError",
                "Authorization check failed",
                "Missing required authorization or insufficient permissions",
                "Ensure proper authorization is provided before calling the function",
            ),
            (
                7,
                "MathError",
                "Mathematical operation failed",
                "Division by zero, overflow, or invalid mathematical operation",
                "Add bounds checking, validate inputs, and handle edge cases in math operations",
            ),
            (
                8,
                "ArrayError",
                "Array operation failed",
                "Index out of bounds, invalid array access, or array size exceeded",
                "Validate array indices before access and check array bounds",
            ),
            (
                9,
                "StringError",
                "String operation failed",
                "Invalid string encoding, length exceeded, or malformed string",
                "Validate string encoding and length before operations",
            ),
            (
                10,
                "MapError",
                "Map operation failed",
                "Key not found, invalid key type, or map operation failed",
                "Check if key exists before access and validate key types",
            ),
        ];

        for (code, name, desc, cause, fix) in std_errs {
            self.standard_errors.insert(
                code,
                ErrorExplanation {
                    code,
                    name: name.to_string(),
                    description: desc.to_string(),
                    common_cause: cause.to_string(),
                    suggested_fix: fix.to_string(),
                },
            );
        }
    }

    pub fn lookup(&self, code: u32) -> Option<&ErrorExplanation> {
        self.custom_errors
            .get(&code)
            .or_else(|| self.standard_errors.get(&code))
    }

    pub fn add_custom_error(&mut self, error: ErrorExplanation) {
        self.custom_errors.insert(error.code, error);
    }

    pub fn load_custom_errors_from_wasm(&mut self, wasm_bytes: &[u8]) -> Result<(), String> {
        let custom_errors = crate::utils::wasm::parse_custom_errors(wasm_bytes)
            .map_err(|e| format!("Failed to parse custom errors from WASM: {:?}", e))?;

        for err in custom_errors {
            self.add_custom_error(ErrorExplanation {
                code: err.code,
                name: err.name,
                description: err.doc.clone(),
                common_cause: "Contract-specific error condition".to_string(),
                suggested_fix: "Review contract documentation or source code".to_string(),
            });
        }
        Ok(())
    }

    pub fn display_error(&self, code: u32) {
        if let Some(explanation) = self.lookup(code) {
            crate::logging::log_display(
                "\n=== Error Explanation ===",
                crate::logging::LogLevel::Info,
            );
            crate::logging::log_display(
                format!("Error Code: {}", explanation.code),
                crate::logging::LogLevel::Info,
            );
            crate::logging::log_display(
                format!("Error Name: {}", explanation.name),
                crate::logging::LogLevel::Info,
            );
            crate::logging::log_display("\nDescription:", crate::logging::LogLevel::Info);
            crate::logging::log_display(
                format!("  {}", explanation.description),
                crate::logging::LogLevel::Info,
            );
            crate::logging::log_display("\nCommon Cause:", crate::logging::LogLevel::Info);
            crate::logging::log_display(
                format!("  {}", explanation.common_cause),
                crate::logging::LogLevel::Info,
            );
            crate::logging::log_display("\nSuggested Fix:", crate::logging::LogLevel::Info);
            crate::logging::log_display(
                format!("  {}", explanation.suggested_fix),
                crate::logging::LogLevel::Info,
            );
            crate::logging::log_display("", crate::logging::LogLevel::Info);
        } else {
            crate::logging::log_display(
                format!("\n=== Error Code: {} ===", code),
                crate::logging::LogLevel::Info,
            );
            crate::logging::log_display(
                "No explanation available for this error code.",
                crate::logging::LogLevel::Info,
            );
            crate::logging::log_display(
                "This may be a custom contract error. Check contract documentation.",
                crate::logging::LogLevel::Info,
            );
            crate::logging::log_display("", crate::logging::LogLevel::Info);
        }
    }
}

impl Default for ErrorDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_error_lookup() {
        let db = ErrorDatabase::new();
        let auth_error = db.lookup(6).expect("Should find standard AuthError");
        assert_eq!(auth_error.name, "AuthError");
        assert_eq!(auth_error.code, 6);

        let missing = db.lookup(999);
        assert!(missing.is_none());
    }

    #[test]
    fn test_custom_error_addition() {
        let mut db = ErrorDatabase::new();
        db.add_custom_error(ErrorExplanation {
            code: 1001,
            name: "MyCustomError".to_string(),
            description: "Custom doc".to_string(),
            common_cause: "Cause".to_string(),
            suggested_fix: "Fix".to_string(),
        });

        let err = db.lookup(1001).expect("Should find custom error");
        assert_eq!(err.name, "MyCustomError");
    }
}
