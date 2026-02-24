use criterion::{black_box, criterion_group, criterion_main, Criterion};
use soroban_debugger::debugger::breakpoint::BreakpointManager;
use soroban_debugger::inspector::CallStackInspector;

fn bench_state_management(c: &mut Criterion) {
    let mut bp_manager = BreakpointManager::new();
    for i in 0..100 {
        bp_manager.add(&format!("func_{}", i));
    }

    let mut group = c.benchmark_group("state_management");

    group.bench_function("breakpoint_check_100_set", |b| {
        b.iter(|| {
            black_box(bp_manager.should_break("func_50"));
        })
    });

    let mut stack = CallStackInspector::new();
    group.bench_function("call_stack_push_pop", |b| {
        b.iter(|| {
            stack.push("test_func".to_string(), Some("contract_id".to_string()));
            black_box(stack.pop());
        })
    });

    group.finish();
}

criterion_group!(benches, bench_state_management);
criterion_main!(benches);
