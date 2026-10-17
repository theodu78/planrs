//! Budget de performance du moteur (spécification J2 § 6, ADR-0009 A18).
//!
//! `schedule` sur le grand réseau (5 000 tâches, 10 000 liens, compilation
//! des calendriers comprise) : 3 exécutions de chauffe, puis médiane de 15.
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

use planrs_core::{Diagnostic, Severity, TaskOutcome, TaskRole, schedule};
use planrs_tests::generate::large_network;

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

#[test]
#[ignore = "budget mesuré en release, job CI perf"]
fn schedule_budget_5000_tasks_10000_links() {
    let p = large_network(2026);
    for _ in 0..3 {
        black_box(schedule(black_box(&p)));
    }
    let mut times: Vec<Duration> = (0..15)
        .map(|_| {
            let t = Instant::now();
            black_box(schedule(black_box(&p)));
            t.elapsed()
        })
        .collect();
    times.sort_unstable();
    let median = times[times.len() / 2];
    eprintln!(
        "schedule (5 000 tâches, 10 000 liens) : médiane {median:?}, min {:?}, max {:?}",
        times[0],
        times[times.len() - 1]
    );
    if median > WARN {
        eprintln!("avertissement : médiane {median:?} au-delà de {WARN:?} (cible locale ≤ 10 ms)");
    }
    assert!(
        median <= BUDGET,
        "médiane {median:?} au-delà du budget {BUDGET:?}"
    );
}
