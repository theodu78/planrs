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
use planrs_core::calendar::Calendar;
use planrs_core::{
    Assignment, AssignmentUid, CalendarUid, Minutes, Project, Resource, ResourceUid, Units, Work, schedule,
};
use planrs_tests::generate::{chain, fan_in, large_network};

/// Grand réseau (5 000 tâches, 10 000 liens) dont chaque tâche reçoit deux
/// affectations de travail (10 000 au total, ADR-0013) : 20 ressources, une
/// sur deux sur un calendrier propre dérivé du Standard, une affectation
/// sur dix décalée d'un jour ; les feuilles automatiques à deux calendriers
/// effectifs sont placées par leurs parts (R-173).
fn assigned_network(seed: u64) -> Project {
    let mut p = large_network(seed);
    for k in 1..=20u32 {
        let mut r = Resource::new(ResourceUid::new(k), format!("R{k}"));
        if k % 2 == 0 {
            let cal = CalendarUid::new(100 + k);
            p.calendars
                .push(Calendar::new_derived(cal, format!("R{k}"), CalendarUid::new(1)))
                .unwrap();
            r.calendar = Some(cal);
        }
        p.resources.push(r).unwrap();
    }
    let tasks: Vec<_> = p.tasks.iter().map(|t| (t.uid(), t.duration.minutes())).collect();
    let mut next = 1u32;
    for (i, (task, d)) in tasks.into_iter().enumerate() {
        for j in 0..2u32 {
            let k = u32::try_from(i % 20).unwrap() + 1;
            let res = ResourceUid::new((k + j * 7 - 1) % 20 + 1);
            let mut a = Assignment::new(AssignmentUid::new(next), task, Some(res));
            a.units = Units::FULL;
            a.work = Work::new(d).unwrap();
            if next.is_multiple_of(10) {
                a.delay = Minutes::new(480);
            }
            p.assignments.push(a).unwrap();
            next += 1;
        }
    }
    p
}

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
    // J5 (lot L4a) : budget visé ≤ 50 ms en release (médiane), informatif.
    let assigned = assigned_network(2026);
    c.bench_function("schedule/5000_assigned", |b| {
        b.iter(|| schedule(black_box(&assigned)));
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
