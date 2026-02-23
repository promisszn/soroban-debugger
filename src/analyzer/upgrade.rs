use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// WASM value type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WasmType {
    I32,
    I64,
    F32,
    F64,
    V128,
    FuncRef,
    ExternRef,
    Unknown,
}

impl fmt::Display for WasmType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WasmType::I32 => write!(f, "i32"),
            WasmType::I64 => write!(f, "i64"),
            WasmType::F32 => write!(f, "f32"),
            WasmType::F64 => write!(f, "f64"),
            WasmType::V128 => write!(f, "v128"),
            WasmType::FuncRef => write!(f, "funcref"),
            WasmType::ExternRef => write!(f, "externref"),
            WasmType::Unknown => write!(f, "?"),
        }
    }
}

/// A function signature extracted from a WASM module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<WasmType>,
    pub results: Vec<WasmType>,
}

impl fmt::Display for FunctionSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params: Vec<String> = self.params.iter().map(|t| t.to_string()).collect();
        let results: Vec<String> = self.results.iter().map(|t| t.to_string()).collect();
        write!(f, "{}({}) -> [{}]", self.name, params.join(", "), results.join(", "))
    }
}

/// A breaking change detected between two contract versions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BreakingChange {
    FunctionRemoved {
        name: String,
    },
    ParameterCountChanged {
        name: String,
        old_count: usize,
        new_count: usize,
    },
    ParameterTypeChanged {
        name: String,
        index: usize,
        old_type: WasmType,
        new_type: WasmType,
    },
    ReturnTypeChanged {
        name: String,
        old_types: Vec<WasmType>,
        new_types: Vec<WasmType>,
    },
}

impl fmt::Display for BreakingChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BreakingChange::FunctionRemoved { name } => {
                write!(f, "[REMOVED] {}", name)
            }
            BreakingChange::ParameterCountChanged { name, old_count, new_count } => {
                write!(f, "[PARAMS_CHANGED] {}: {} params -> {} params", name, old_count, new_count)
            }
            BreakingChange::ParameterTypeChanged { name, index, old_type, new_type } => {
                write!(f, "[PARAM_TYPE] {} param[{}]: {} -> {}", name, index, old_type, new_type)
            }
            BreakingChange::ReturnTypeChanged { name, old_types, new_types } => {
                let old: Vec<String> = old_types.iter().map(|t| t.to_string()).collect();
                let new: Vec<String> = new_types.iter().map(|t| t.to_string()).collect();
                write!(f, "[RETURN_TYPE] {}: [{}] -> [{}]", name, old.join(", "), new.join(", "))
            }
        }
    }
}

/// A non-breaking change detected between two contract versions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NonBreakingChange {
    FunctionAdded { name: String },
}

impl fmt::Display for NonBreakingChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NonBreakingChange::FunctionAdded { name } => write!(f, "[ADDED] {}", name),
        }
    }
}

/// Execution result comparison when --test-inputs is provided
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionDiff {
    pub function: String,
    pub args: String,
    pub old_result: String,
    pub new_result: String,
    pub outputs_match: bool,
}

/// The full compatibility report
#[derive(Debug, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub is_compatible: bool,
    pub old_wasm_path: String,
    pub new_wasm_path: String,
    pub breaking_changes: Vec<BreakingChange>,
    pub non_breaking_changes: Vec<NonBreakingChange>,
    pub old_functions: Vec<FunctionSignature>,
    pub new_functions: Vec<FunctionSignature>,
    pub execution_diffs: Vec<ExecutionDiff>,
}

pub struct UpgradeAnalyzer;

impl UpgradeAnalyzer {
    /// Analyze two WASM binaries and produce a compatibility report
    pub fn analyze(
        old_wasm: &[u8],
        new_wasm: &[u8],
        old_path: &str,
        new_path: &str,
        execution_diffs: Vec<ExecutionDiff>,
    ) -> Result<CompatibilityReport> {
        let old_functions = crate::utils::wasm::parse_function_signatures(old_wasm)?;
        let new_functions = crate::utils::wasm::parse_function_signatures(new_wasm)?;

        let (breaking_changes, non_breaking_changes) =
            Self::diff_signatures(&old_functions, &new_functions);

        let has_execution_mismatches = execution_diffs.iter().any(|d| !d.outputs_match);
        let is_compatible = breaking_changes.is_empty() && !has_execution_mismatches;

        Ok(CompatibilityReport {
            is_compatible,
            old_wasm_path: old_path.to_string(),
            new_wasm_path: new_path.to_string(),
            breaking_changes,
            non_breaking_changes,
            old_functions,
            new_functions,
use crate::runtime::executor::ContractExecutor;
use crate::Result;
use std::collections::HashMap;
use std::fmt::Write;
use wasmparser::{Parser, Payload};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<String>,
    pub results: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SignatureDiff {
    pub added: Vec<FunctionSignature>,
    pub removed: Vec<FunctionSignature>,
    pub changed: Vec<(FunctionSignature, FunctionSignature)>, // (old, new)
}

#[derive(Debug, Clone)]
pub struct ExecutionDiff {
    pub function_name: String,
    pub old_output: String,
    pub new_output: String,
    pub output_match: bool,
}

#[derive(Debug, Clone, Default)]
pub struct StorageDiff {
    // Placeholder for now
}

#[derive(Debug, Clone, Default)]
pub struct CompatibilityReport {
    pub signature_diff: SignatureDiff,
    pub storage_diff: StorageDiff,
    pub execution_diffs: Vec<ExecutionDiff>,
}

#[derive(Debug, Default)]
pub struct UpgradeAnalyzer;

impl UpgradeAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(
        &self,
        old_wasm: &[u8],
        new_wasm: &[u8],
        function: Option<&str>,
        args: Option<&str>,
    ) -> Result<CompatibilityReport> {
        let old_signatures = self.parse_exported_signatures(old_wasm)?;
        let new_signatures = self.parse_exported_signatures(new_wasm)?;

        let signature_diff = self.diff_signatures(&old_signatures, &new_signatures);

        let mut execution_diffs = Vec::new();
        let storage_diff = StorageDiff::default();

        if let Some(func_name) = function {
            let diff = self.run_side_by_side(old_wasm, new_wasm, func_name, args)?;
            execution_diffs.push(diff);
        }

        Ok(CompatibilityReport {
            signature_diff,
            storage_diff,
            execution_diffs,
        })
    }

    /// Compute breaking and non-breaking changes between two sets of function signatures
    fn diff_signatures(
        old: &[FunctionSignature],
        new: &[FunctionSignature],
    ) -> (Vec<BreakingChange>, Vec<NonBreakingChange>) {
        let mut breaking = Vec::new();
        let mut non_breaking = Vec::new();

        let new_map: HashMap<&str, &FunctionSignature> =
            new.iter().map(|s| (s.name.as_str(), s)).collect();
        let old_names: std::collections::HashSet<&str> =
            old.iter().map(|s| s.name.as_str()).collect();

        // Check old functions against new
        for old_sig in old {
            match new_map.get(old_sig.name.as_str()) {
                None => {
                    breaking.push(BreakingChange::FunctionRemoved {
                        name: old_sig.name.clone(),
                    });
                }
                Some(new_sig) => {
                    // Check parameter count
                    if old_sig.params.len() != new_sig.params.len() {
                        breaking.push(BreakingChange::ParameterCountChanged {
                            name: old_sig.name.clone(),
                            old_count: old_sig.params.len(),
                            new_count: new_sig.params.len(),
                        });
                    } else {
                        // Check per-parameter types
                        for (i, (old_t, new_t)) in
                            old_sig.params.iter().zip(new_sig.params.iter()).enumerate()
                        {
                            if old_t != new_t {
                                breaking.push(BreakingChange::ParameterTypeChanged {
                                    name: old_sig.name.clone(),
                                    index: i,
                                    old_type: old_t.clone(),
                                    new_type: new_t.clone(),
                                });
                            }
                        }
                    }

                    // Check return types
                    if old_sig.results != new_sig.results {
                        breaking.push(BreakingChange::ReturnTypeChanged {
                            name: old_sig.name.clone(),
                            old_types: old_sig.results.clone(),
                            new_types: new_sig.results.clone(),
                        });
                    }
                }
            }
        }

        // Check for newly added functions
        for new_sig in new {
            if !old_names.contains(new_sig.name.as_str()) {
                non_breaking.push(NonBreakingChange::FunctionAdded {
                    name: new_sig.name.clone(),
                });
            }
        }

        (breaking, non_breaking)
    pub fn parse_exported_signatures(&self, wasm_bytes: &[u8]) -> Result<Vec<FunctionSignature>> {
        let mut signatures = Vec::new();
        let parser = Parser::new(0);

        // We need to correlate function indices with types and exports
        let mut type_definitions = Vec::new();
        let mut function_types = Vec::new(); // maps function index -> type index
        let mut exports = Vec::new();

        for payload in parser.parse_all(wasm_bytes) {
            match payload.map_err(|e| {
                crate::DebuggerError::WasmLoadError(format!("Failed to parse WASM: {}", e))
            })? {
                Payload::TypeSection(reader) => {
                    for rec_group in reader {
                        let rec_group = rec_group.map_err(|e| {
                            crate::DebuggerError::WasmLoadError(format!(
                                "Failed to read type section: {}",
                                e
                            ))
                        })?;
                        for ty in rec_group.types() {
                            if let wasmparser::CompositeType::Func(func_type) = &ty.composite_type {
                                type_definitions.push(func_type.clone());
                            }
                        }
                    }
                }
                Payload::FunctionSection(reader) => {
                    for type_idx in reader {
                        function_types.push(type_idx.map_err(|e| {
                            crate::DebuggerError::WasmLoadError(format!(
                                "Failed to read function section: {}",
                                e
                            ))
                        })?);
                    }
                }
                Payload::ExportSection(reader) => {
                    for export in reader {
                        let export = export.map_err(|e| {
                            crate::DebuggerError::WasmLoadError(format!(
                                "Failed to read export section: {}",
                                e
                            ))
                        })?;
                        if let wasmparser::ExternalKind::Func = export.kind {
                            exports.push((export.name.to_string(), export.index));
                        }
                    }
                }
                _ => {}
            }
        }

        for (name, func_idx) in exports {
            if let Some(&type_idx) = function_types.get(func_idx as usize) {
                if let Some(func_type) = type_definitions.get(type_idx as usize) {
                    let params = func_type
                        .params()
                        .iter()
                        .map(|t| format!("{:?}", t))
                        .collect();
                    let results = func_type
                        .results()
                        .iter()
                        .map(|t| format!("{:?}", t))
                        .collect();

                    signatures.push(FunctionSignature {
                        name,
                        params,
                        results,
                    });
                }
            }
        }

        Ok(signatures)
    }

    pub fn diff_signatures(
        &self,
        old_sigs: &[FunctionSignature],
        new_sigs: &[FunctionSignature],
    ) -> SignatureDiff {
        let old_map: HashMap<String, FunctionSignature> = old_sigs
            .iter()
            .map(|s| (s.name.clone(), s.clone()))
            .collect();

        let new_map: HashMap<String, FunctionSignature> = new_sigs
            .iter()
            .map(|s| (s.name.clone(), s.clone()))
            .collect();

        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();

        // Check for removed and changed
        for (name, old_sig) in &old_map {
            if let Some(new_sig) = new_map.get(name) {
                if old_sig != new_sig {
                    changed.push((old_sig.clone(), new_sig.clone()));
                }
            } else {
                removed.push(old_sig.clone());
            }
        }

        // Check for added
        for (name, new_sig) in &new_map {
            if !old_map.contains_key(name) {
                added.push(new_sig.clone());
            }
        }

        SignatureDiff {
            added,
            removed,
            changed,
        }
    }

    pub fn run_side_by_side(
        &self,
        old_wasm: &[u8],
        new_wasm: &[u8],
        function: &str,
        args: Option<&str>,
    ) -> Result<ExecutionDiff> {
        let mut old_executor = ContractExecutor::new(old_wasm.to_vec())?;
        let mut new_executor = ContractExecutor::new(new_wasm.to_vec())?;

        // Run old
        let old_result = old_executor.execute(function, args);
        let old_output = match &old_result {
            Ok(v) => v.clone(),
            Err(e) => format!("Error: {}", e),
        };

        // Run new
        let new_result = new_executor.execute(function, args);
        let new_output = match &new_result {
            Ok(v) => v.clone(),
            Err(e) => format!("Error: {}", e),
        };

        // Compare outcomes
        let output_match = old_output == new_output;

        Ok(ExecutionDiff {
            function_name: function.to_string(),
            old_output,
            new_output,
            output_match,
        })
    }

    pub fn generate_markdown_report(&self, report: &CompatibilityReport) -> String {
        let mut out = String::new();

        writeln!(out, "# Upgrade Compatibility Report").unwrap();
        writeln!(out).unwrap();

        let sig_diff = &report.signature_diff;

        let mut breaking = false;
        if !sig_diff.removed.is_empty() || !sig_diff.changed.is_empty() {
            breaking = true;
        }
        for exec in &report.execution_diffs {
            if !exec.output_match {
                breaking = true;
            }
        }

        if breaking {
            writeln!(out, "## ⚠️ Breaking Changes Detected").unwrap();
        } else {
            writeln!(out, "## ✅ Compatible").unwrap();
        }
        writeln!(out).unwrap();

        // Signatures
        if !sig_diff.removed.is_empty() {
            writeln!(out, "### Removed Functions").unwrap();
            for sig in &sig_diff.removed {
                writeln!(out, "- `{}`", sig.name).unwrap();
            }
            writeln!(out).unwrap();
        }

        if !sig_diff.changed.is_empty() {
            writeln!(out, "### Changed Functions").unwrap();
            for (old, new) in &sig_diff.changed {
                writeln!(out, "- `{}`", old.name).unwrap();
                writeln!(out, "  - Old: `{:?} -> {:?}`", old.params, old.results).unwrap();
                writeln!(out, "  - New: `{:?} -> {:?}`", new.params, new.results).unwrap();
            }
            writeln!(out).unwrap();
        }

        if !sig_diff.added.is_empty() {
            writeln!(out, "### Added Functions").unwrap();
            for sig in &sig_diff.added {
                writeln!(out, "- `{}`", sig.name).unwrap();
            }
            writeln!(out).unwrap();
        }

        // Execution
        if !report.execution_diffs.is_empty() {
            writeln!(out, "### Execution Comparison").unwrap();
            for diff in &report.execution_diffs {
                writeln!(out, "#### Function: `{}`", diff.function_name).unwrap();
                if diff.output_match {
                    writeln!(out, "- ✅ Outputs match: `{}`", diff.old_output).unwrap();
                } else {
                    writeln!(out, "- ❌ Outputs differ").unwrap();
                    writeln!(out, "  - Old: `{}`", diff.old_output).unwrap();
                    writeln!(out, "  - New: `{}`", diff.new_output).unwrap();
                }
                writeln!(out).unwrap();
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_signatures_no_changes() {
        let analyzer = UpgradeAnalyzer::new();
        let sig = FunctionSignature {
            name: "test".to_string(),
            params: vec!["I32".to_string()],
            results: vec!["Val".to_string()],
        };
        let diff = analyzer.diff_signatures(std::slice::from_ref(&sig), std::slice::from_ref(&sig));
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert!(diff.changed.is_empty());
    }

    #[test]
    fn test_diff_signatures_add_remove() {
        let analyzer = UpgradeAnalyzer::new();
        let sig1 = FunctionSignature {
            name: "foo".into(),
            params: vec![],
            results: vec![],
        };
        let sig2 = FunctionSignature {
            name: "bar".into(),
            params: vec![],
            results: vec![],
        };

        let diff =
            analyzer.diff_signatures(std::slice::from_ref(&sig1), std::slice::from_ref(&sig2));

        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].name, "foo");
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].name, "bar");
    }
}
