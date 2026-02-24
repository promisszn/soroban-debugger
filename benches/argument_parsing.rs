use criterion::{black_box, criterion_group, criterion_main, Criterion};
use soroban_debugger::utils::arguments::ArgumentParser;
use soroban_sdk::Env;

fn bench_argument_parsing(c: &mut Criterion) {
    let env = Env::default();
    let _parser = ArgumentParser::new(env);

    let mut group = c.benchmark_group("argument_parsing");

    group.bench_function("simple_types", |b| {
        let json = r#"[42, "hello", true]"#;
        b.iter(|| {
            let env = Env::default();
            let parser = ArgumentParser::new(env);
            let result = parser.parse_args_string(black_box(json)).unwrap();
            black_box(result);
        })
    });

    group.bench_function("complex_types", |b| {
        let json = r#"[
            {"type": "u128", "value": 1000000},
            {"type": "i128", "value": -5000},
            {"type": "symbol", "value": "test_symbol"},
            {"type": "string", "value": "test string"},
            {"type": "option", "value": 42},
            {"type": "option", "value": null},
            {"type": "tuple", "arity": 3, "value": [1, "two", false]},
            {"user": "alice", "amount": 100, "active": true}
        ]"#;
        b.iter(|| {
            let env = Env::default();
            let parser = ArgumentParser::new(env);
            let result = parser.parse_args_string(black_box(json)).unwrap();
            black_box(result);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_argument_parsing);
criterion_main!(benches);
