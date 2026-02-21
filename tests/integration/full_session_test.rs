use assert_cmd::Command;

#[test]
fn test_full_debug_session_walkthrough() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let wasm_path = std::path::PathBuf::from(manifest_dir)
        .join("tests")
        .join("fixtures")
        .join("wasm")
        .join("counter.wasm");

    // Skip if fixture not built
    if !wasm_path.exists() {
        eprintln!("Skipping test: counter.wasm fixture not found.");
        return;
    }

    // Initialize the command: run counter entry point with instruction stepping
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_soroban-debug"));
    cmd.arg("run")
        .arg("--contract")
        .arg(&wasm_path)
        .arg("--function")
        .arg("increment")
        .arg("--breakpoint")
        .arg("increment")
        .arg("--instruction-debug")
        .arg("--step-instructions");

    // Spawn the command with piped stdin
    let mut child = cmd.spawn().expect("Failed to spawn soroban-debug");
    let mut stdin = child.stdin.take().expect("Failed to open stdin");

    // Simulate session:
    // 1. Step into (s)
    // 2. View info (i)
    // 3. Continue to completion (c)
    writeln!(stdin, "s").unwrap();
    writeln!(stdin, "i").unwrap();
    writeln!(stdin, "c").unwrap();

    // Capture output
    let output = child.wait_with_output().expect("Failed to wait for output");
    assert!(output.status.success());
}
