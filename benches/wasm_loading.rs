use criterion::{black_box, criterion_group, criterion_main, Criterion};
use soroban_debugger::ContractExecutor;
use std::fs;
use std::path::PathBuf;

fn bench_wasm_loading(c: &mut Criterion) {
    let mut wasm_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    wasm_path.push("tests/fixtures/wasm/counter.wasm");
    let wasm_bytes = fs::read(wasm_path).expect("Failed to read counter.wasm");

    c.bench_function("wasm_loading_counter", |b| {
        b.iter(|| {
            // ContractExecutor::new performs the full loading pipeline including
            // environment setup, contract registration, and spec parsing.
            let executor = ContractExecutor::new(black_box(wasm_bytes.clone())).unwrap();
            black_box(executor);
        })
    });
}

criterion_group!(benches, bench_wasm_loading);
criterion_main!(benches);
