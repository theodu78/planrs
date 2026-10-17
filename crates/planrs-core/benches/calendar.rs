//! Benchmark informatif des calendriers compilés (ADR-0008, § 9.4).
//!
//! Scénario : calendrier Standard avec 20 jours fériés par an sur 5 ans
//! (2026-2030), compilé, puis 10 000 appels de `add_working` et de
//! `working_between` sur des instants et des durées pseudo-aléatoires. Le
//! générateur est déterministe (congruence linéaire à graine fixe) : aucune
//! dépendance d'aléa.
//!
//! Cibles informatives : compilation < 50 µs ; conversion < 300 ns en
//! moyenne. Non bloquant et non exécuté en CI (seulement compilé par
//! `cargo clippy --all-targets`) : `cargo bench -p planrs-core`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "benchmark : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use planrs_core::calendar::{
    Calendar, CalendarException, CompiledCalendar, DaySetting, Recurrence, standard,
};
use planrs_core::{Arena, CalendarUid, Day, DayRange, Minutes, Moment};

/// Nombre de conversions par itération.
const CALLS: usize = 10_000;

/// Congruence linéaire (constantes de Knuth, MMIX) : déterministe.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    /// Entier dans `[lo, hi]`.
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        let span = u64::try_from(hi - lo + 1).unwrap();
        lo + i64::try_from(self.next() % span).unwrap()
    }
}

/// Standard avec 20 jours fériés par an, de 2026 à 2030.
fn calendars() -> Arena<CalendarUid, Calendar> {
    let mut c = standard(CalendarUid::new(1), "Standard".to_owned());
    let mut rng = Lcg(2026);
    for year in 2026..=2030 {
        let first = format!("{year}-01-01").parse::<Day>().unwrap().index();
        let mut added = 0;
        while added < 20 {
            let d = Day::new(i32::try_from(rng.range(i64::from(first), i64::from(first) + 364)).unwrap())
                .unwrap();
            let e = CalendarException {
                name: format!("Férié {added}"),
                period: DayRange::single(d),
                setting: DaySetting::NonWorking,
                recurrence: Recurrence::Period,
                occurrences: None,
            };
            if c.add_exception(e).is_ok() {
                added += 1;
            }
        }
    }
    Arena::try_from(vec![c]).unwrap()
}

/// Instants et durées pseudo-aléatoires sur 2026-2030.
fn inputs() -> Vec<(Moment, Minutes, Moment)> {
    let lo = "2026-01-01T00:00".parse::<Moment>().unwrap().minutes();
    let hi = "2030-12-31T00:00".parse::<Moment>().unwrap().minutes();
    let mut rng = Lcg(42);
    (0..CALLS)
        .map(|_| {
            let a = Moment::from_minutes(rng.range(lo, hi)).unwrap();
            let b = Moment::from_minutes(rng.range(lo, hi)).unwrap();
            (a, Minutes::new(rng.range(-50_000, 50_000)), b)
        })
        .collect()
}

fn bench(c: &mut Criterion) {
    let cals = calendars();
    let uid = CalendarUid::new(1);
    c.bench_function("compile Standard + 100 fériés", |b| {
        b.iter(|| CompiledCalendar::compile(black_box(&cals), uid).unwrap());
    });
    let compiled = CompiledCalendar::compile(&cals, uid).unwrap();
    let inputs = inputs();
    c.bench_function("add_working × 10 000", |b| {
        b.iter(|| {
            for (t, d, _) in &inputs {
                black_box(compiled.add_working(black_box(*t), black_box(*d)).ok());
            }
        });
    });
    c.bench_function("working_between × 10 000", |b| {
        b.iter(|| {
            for (a, _, bb) in &inputs {
                black_box(compiled.working_between(black_box(*a), black_box(*bb)).ok());
            }
        });
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
