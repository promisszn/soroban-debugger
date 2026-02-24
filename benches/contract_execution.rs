use criterion::{black_box, criterion_group, criterion_main, Criterion};
use soroban_debugger::runtime::executor::ContractExecutor;
use std::fs;
use std::path::PathBuf;

fn bench_contract_execution(c: &mut Criterion) {
    let mut wasm_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    wasm_path.push("tests/fixtures/wasm/counter.wasm");
    let wasm_bytes = fs::read(wasm_path).expect("Failed to read counter.wasm");

    // Setup executor once for the execution benchmarks
    let mut executor = ContractExecutor::new(wasm_bytes).unwrap();

    let mut group = c.benchmark_group("contract_execution");

    group.bench_function("counter_increment", |b| {
        b.iter(|| {
            executor.env().budget().reset_unlimited();
            let result = executor
                .execute(black_box("increment"), black_box(None))
                .unwrap();
            black_box(result);
        })
    });

    group.bench_function("counter_get", |b| {
        b.iter(|| {
            executor.env().budget().reset_unlimited();
            let result = executor.execute(black_box("get"), black_box(None)).unwrap();
            black_box(result);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_contract_execution);
criterion_main!(benches);
