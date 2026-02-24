use criterion::{black_box, criterion_group, criterion_main, Criterion};
use soroban_debugger::inspector::storage::StorageInspector;
use soroban_env_host::budget::AsBudget;
use soroban_env_host::xdr::{
    ContractDataDurability, ContractDataEntry, ExtensionPoint, LedgerEntry, LedgerEntryData,
    LedgerEntryExt, LedgerKey, LedgerKeyContractData, ScAddress, ScSymbol, ScVal,
};
use soroban_env_host::Host;
use std::collections::HashMap;
use std::rc::Rc;

fn bench_storage_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_ops");

    // Benchmark diff computation
    let mut before = HashMap::new();
    let mut after = HashMap::new();
    for i in 0..1000 {
        let key = format!("contract_data:Persistent:Symbol(key_{:04})", i);
        let value = format!("I32({})", i);
        before.insert(key.clone(), value.clone());
        after.insert(key, value);
    }
    // Modify 10%
    for i in 0..100 {
        after.insert(
            format!("contract_data:Persistent:Symbol(key_{:04})", i),
            format!("I32({})", i + 1),
        );
    }
    // Delete 5%
    for i in 900..950 {
        after.remove(&format!("contract_data:Persistent:Symbol(key_{:04})", i));
    }
    // Add 5%
    for i in 1000..1050 {
        after.insert(
            format!("contract_data:Persistent:Symbol(key_{:04})", i),
            format!("I32({})", i),
        );
    }

    group.bench_function("compute_diff_1000_entries", |b| {
        b.iter(|| {
            let diff = StorageInspector::compute_diff(
                black_box(&before),
                black_box(&after),
                black_box(&[]),
            );
            black_box(diff);
        })
    });

    // Benchmark snapshot capture
    let host = Host::default();
    let contract_id = [0u8; 32];
    let address = ScAddress::Contract(contract_id.into());

    host.with_mut_storage(|storage| {
        for i in 0..1000 {
            let key_val = ScVal::Symbol(ScSymbol::try_from(format!("key_{:04}", i)).unwrap());
            let key = LedgerKey::ContractData(LedgerKeyContractData {
                contract: address.clone(),
                key: key_val.clone(),
                durability: ContractDataDurability::Persistent,
            });
            let entry = LedgerEntry {
                last_modified_ledger_seq: 1,
                data: LedgerEntryData::ContractData(ContractDataEntry {
                    contract: address.clone(),
                    key: key_val,
                    durability: ContractDataDurability::Persistent,
                    val: ScVal::I32(i as i32),
                    ext: ExtensionPoint::V0,
                }),
                ext: LedgerEntryExt::V0,
            };
            storage
                .map
                .insert(Rc::new(key), Some((Rc::new(entry), None)), host.as_budget())
                .unwrap();
        }
        Ok(())
    })
    .unwrap();

    group.bench_function("capture_snapshot_1000_entries", |b| {
        b.iter(|| {
            host.as_budget().reset_unlimited().unwrap();
            let snapshot = StorageInspector::capture_snapshot(black_box(&host));
            black_box(snapshot);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_storage_ops);
criterion_main!(benches);
