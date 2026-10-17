//! Budget de performance du moteur (spécification J2 § 6, ADR-0009 A18).
//!
//! `schedule` sur le grand réseau (5 000 tâches, 10 000 liens, compilation
//! des calendriers comprise), et sur ses variantes J3 (spécification J3
//! § 9.2) : `5000_constrained` (20 % de contraintes datées, 5 % d'ALAP, 5 %
//! d'échéances) et `5000_from_finish` (planification à rebours, mêmes
//! proportions) : 3 exécutions de chauffe, puis médiane de 15.
//! Échec au-delà de 50 ms, avertissement au-delà de 25 ms ; cible locale
//! ≤ 10 ms. Mesure seulement en release, par le job CI `perf` :
//! `cargo test -p planrs-tests --release --test perf -- --ignored --nocapture`.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::print_stderr,
    reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
)]

use std::hint::black_box;
use std::time::{Duration, Instant};

use planrs_core::{Diagnostic, Project, Severity, TaskOutcome, TaskRole, schedule};
use planrs_tests::generate::{constrained_network, from_finish_network, large_network};

/// Seuil d'échec de la médiane.
const BUDGET: Duration = Duration::from_millis(50);
/// Seuil d'avertissement.
const WARN: Duration = Duration::from_millis(25);

#[test]
fn large_network_shape() {
    // Le réseau mesuré a la forme annoncée et se calcule sans erreur.
    let p = large_network(2026);
    assert_eq!(p.tasks.len(), 5_000);
    let links: usize = p.tasks.iter().map(|t| t.predecessors.len()).sum();
    assert_eq!(links, 10_000);
    let backward = p
        .tasks
        .iter()
        .flat_map(|t| t.predecessors.iter().map(move |l| (l.predecessor, t.uid())))
        .filter(|(pred, succ)| pred > succ)
        .count();
    // Environ 20 % des liens remontent la liste (chemin de Kahn forcé).
    assert!((1_500..=2_500).contains(&backward), "{backward} liens remontants");
    let s = schedule(&p);
    let summaries = s
        .tasks
        .values()
        .filter(|o| matches!(o, TaskOutcome::Scheduled(ts) if ts.role == TaskRole::Summary))
        .count();
    assert_eq!(summaries, 525);
    assert!(
        s.tasks
            .values()
            .all(|o| matches!(o, TaskOutcome::Scheduled(ts) if !ts.degraded))
    );
    let errors: Vec<&Diagnostic> = s
        .diagnostics
        .iter()
        .filter(|d| d.severity() == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    // Déterministe.
    assert_eq!(large_network(2026), p);
}

/// Les variantes J3 se calculent sans erreur ni tâche dégradée, avec leurs
/// contraintes (et, à rebours, leur ancre), de façon déterministe.
#[test]
fn constrained_networks_shape() {
    for (name, p) in [
        ("5000_constrained", constrained_network(2026)),
        ("5000_from_finish", from_finish_network(2026)),
    ] {
        let dated = p.tasks.iter().filter(|t| t.constraint.kind().is_dated()).count();
        assert!(
            (700..=1_200).contains(&dated),
            "{name} : {dated} contraintes datées"
        );
        let deadlines = p.tasks.iter().filter(|t| t.deadline.is_some()).count();
        assert!((150..=300).contains(&deadlines), "{name} : {deadlines} échéances");
        let s = schedule(&p);
        assert!(
            s.tasks
                .values()
                .all(|o| matches!(o, TaskOutcome::Scheduled(ts) if !ts.degraded)),
            "{name}"
        );
        let errors: Vec<&Diagnostic> = s
            .diagnostics
            .iter()
            .filter(|d| d.severity() == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "{name} : {errors:?}");
    }
    assert_eq!(constrained_network(2026), constrained_network(2026));
    assert_eq!(from_finish_network(2026), from_finish_network(2026));
}

/// Médiane de 15 exécutions après 3 de chauffe ; échec au-delà du budget.
fn budget(name: &str, p: &Project) {
    for _ in 0..3 {
        black_box(schedule(black_box(p)));
    }
    let mut times: Vec<Duration> = (0..15)
        .map(|_| {
            let t = Instant::now();
            black_box(schedule(black_box(p)));
            t.elapsed()
        })
        .collect();
    times.sort_unstable();
    let median = times[times.len() / 2];
    eprintln!(
        "schedule ({name}) : médiane {median:?}, min {:?}, max {:?}",
        times.first().copied().unwrap_or_default(),
        times.last().copied().unwrap_or_default()
    );
    if median > WARN {
        eprintln!("avertissement : médiane {median:?} au-delà de {WARN:?} (cible locale ≤ 10 ms)");
    }
    assert!(
        median <= BUDGET,
        "{name} : médiane {median:?} au-delà du budget {BUDGET:?}"
    );
}

#[test]
#[ignore = "budget mesuré en release, job CI perf"]
fn schedule_budget_5000_tasks_10000_links() {
    budget("5 000 tâches, 10 000 liens", &large_network(2026));
}

#[test]
#[ignore = "budget mesuré en release, job CI perf"]
fn schedule_budget_5000_constrained() {
    budget("5000_constrained", &constrained_network(2026));
}

#[test]
#[ignore = "budget mesuré en release, job CI perf"]
fn schedule_budget_5000_from_finish() {
    budget("5000_from_finish", &from_finish_network(2026));
}
