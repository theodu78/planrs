//! Benchmark informatif du moteur (spécification J2 § 6), exécuté en CI
//! (job `perf`) sans seuil : le seuil bloquant est le test `tests/perf`.
//! `cargo bench -p planrs-tests --bench schedule`.

#![allow(
    clippy::unwrap_used,
    clippy::arithmetic_side_effects,
    reason = "benchmark : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use planrs_core::schedule;
use planrs_tests::generate::{assigned_network, chain, fan_in, large_network};

fn bench(c: &mut Criterion) {
    let large = large_network(2026);
    c.bench_function("schedule/5000_tasks_10000_links", |b| {
        b.iter(|| schedule(black_box(&large)));
    });
    let long = chain(5_000);
    c.bench_function("schedule/chain_5000", |b| b.iter(|| schedule(black_box(&long))));
    let wide = fan_in(10_000);
    c.bench_function("schedule/10000_links_into_one", |b| {
        b.iter(|| schedule(black_box(&wide)));
    });
    // J5 : budget bloquant de 50 ms dans `tests/perf` (`perf_5000_assigned`).
    let assigned = assigned_network(2026);
    c.bench_function("schedule/5000_assigned", |b| {
        b.iter(|| schedule(black_box(&assigned)));
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
