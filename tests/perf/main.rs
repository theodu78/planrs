//! Budget de performance du moteur (spécification J2 § 6, ADR-0009 A18).
//!
//! `schedule` sur le grand réseau (5 000 tâches, 10 000 liens, compilation
//! des calendriers comprise), et sur ses variantes J3 (spécification J3
//! § 9.2) : `5000_constrained` (20 % de contraintes datées, 5 % d'ALAP, 5 %
//! d'échéances) et `5000_from_finish` (planification à rebours, mêmes
//! proportions) : 3 exécutions de chauffe, puis médiane de 15.
//! Échec au-delà de 50 ms, avertissement au-delà de 25 ms ; cible locale
//! ≤ 10 ms. J5 : `5000_assigned` (10 000 affectations, ADR-0013), même
//! budget de 50 ms. Hiérarchie profonde hostile (ADR-0012, lot L15) : 256 niveaux
//! de récapitulatives imbriquées, 5 000 tâches, 10 000 liens dont des FS,
//! SS et FF depuis chaque récapitulative ; budget 30 ms de médiane.
//! Mesure seulement en release, par le job CI `perf` :
//! `cargo test -p planrs-tests --release --test perf -- --ignored --nocapture`.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::print_stderr,
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
)]

use std::hint::black_box;
use std::time::{Duration, Instant};

use planrs_core::calendar::BuiltinCalendarNames;
use planrs_core::schedule::schedule_reference;
use planrs_core::{
    Diagnostic, Duration as TaskDuration, DurationUnit, Lag, LagValue, Link, LinkType, Minutes, Moment,
    OutlineLevel, Percent, Project, ProjectInit, ScheduleAnchor, Severity, Task, TaskOutcome, TaskRole,
    TaskUid, TimeBasis, schedule,
};
use planrs_tests::generate::{assigned_network, constrained_network, from_finish_network, large_network};

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
    budget_of(name, p, BUDGET);
}

/// [`budget`] avec un seuil explicite.
fn budget_of(name: &str, p: &Project, limit: Duration) {
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
        median <= limit,
        "{name} : médiane {median:?} au-delà du budget {limit:?}"
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

/// Le réseau affecté (J5, ADR-0013) a la forme annoncée : 10 000
/// affectations de travail placées, aucune erreur ni tâche dégradée,
/// charges calculées pour les 20 ressources ; déterministe.
#[test]
fn assigned_network_shape() {
    let p = assigned_network(2026);
    assert_eq!(p.tasks.len(), 5_000);
    assert_eq!(p.assignments.len(), 10_000);
    assert_eq!(p.resources.len(), 20);
    let s = schedule(&p);
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
    assert_eq!(s.resources.len(), 20);
    assert_eq!(s.assignments.len(), 10_000);
    assert_eq!(assigned_network(2026), p);
}

/// Budget J5 (ADR-0013) : 5 000 tâches, 10 000 liens, 10 000 affectations
/// (placement par affectation, cumuls et surutilisation compris), 50 ms.
#[test]
#[ignore = "budget mesuré en release, job CI perf"]
fn perf_5000_assigned() {
    budget("5000_assigned", &assigned_network(2026));
}

/// Profondeur de la hiérarchie hostile.
const DEPTH: u32 = 256;
/// Tâches de la hiérarchie hostile.
const DEEP_TASKS: u32 = 5_000;
/// Liens de la hiérarchie hostile.
const DEEP_LINKS: usize = 10_000;

/// Hiérarchie hostile (lot L15) : récapitulative `S_i` au niveau `i` (1 à
/// 256), chacune suivie de ses feuilles au niveau `i + 1` (environ 17 par
/// niveau, de profondeur moyenne 128) ; puis 256 tâches de premier niveau
/// `Z_i`. Liens : `S_i → Z_i` (FS, +1 h), `S_i → Z_(i+1)` (SS), `S_i →
/// Z_(i+2)` (FF) ; chaque feuille a des prédécesseurs FS ou SS parmi les
/// 40 feuilles qui la précèdent, jusqu'à 10 000 liens. Chaque feuille hérite
/// donc des arcs de tous ses ancêtres : la référence (J3) remonte jusqu'à
/// 256 agrégats et 768 arcs par nœud.
fn deep_hierarchy() -> Project {
    let start: Moment = "2026-11-02T08:00".parse().unwrap();
    let mut p = Project::new(ProjectInit {
        name: "Hiérarchie profonde".to_owned(),
        anchor: ScheduleAnchor::FromStart(start),
        current_date: start,
        calendar_names: BuiltinCalendarNames::default(),
    });
    let options = p.options;
    let day = |n: i64| {
        TaskDuration::new(
            Minutes::new(n * 480),
            DurationUnit::Day,
            TimeBasis::Working,
            false,
        )
        .unwrap()
    };
    let leaves_total = DEEP_TASKS - 2 * DEPTH;
    let mut uid = 0u32;
    let mut spine = Vec::new();
    let mut leaves: Vec<TaskUid> = Vec::new();
    let mut tasks = Vec::new();
    for i in 1..=DEPTH {
        uid += 1;
        let mut s = Task::new(TaskUid::new(uid), format!("S{i}"), &options);
        s.outline_level = OutlineLevel::new(u16::try_from(i).unwrap()).unwrap();
        spine.push(s.uid());
        tasks.push(s);
        let k = leaves_total / DEPTH + u32::from(i <= leaves_total % DEPTH);
        for j in 0..k {
            uid += 1;
            let mut t = Task::new(TaskUid::new(uid), format!("F{i}.{j}"), &options);
            t.outline_level = OutlineLevel::new(u16::try_from(i + 1).unwrap()).unwrap();
            t.duration = day(i64::from(1 + (uid % 5)));
            t.progress.percent_complete = Percent::new(i64::from(uid % 11) * 1_000).unwrap();
            leaves.push(t.uid());
            tasks.push(t);
        }
    }
    let mut links = 0usize;
    // Prédécesseurs des feuilles : jusqu'à 40 en arrière, déterministes.
    let mut leaf_preds: Vec<Vec<Link>> = vec![Vec::new(); leaves.len()];
    let mut x: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = || {
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        x
    };
    let summary_links = 3 * usize::try_from(DEPTH).unwrap();
    while links < DEEP_LINKS - summary_links {
        let s = usize::try_from(next() % u64::try_from(leaves.len()).unwrap()).unwrap();
        if s == 0 {
            continue;
        }
        let back = 1 + usize::try_from(next() % 40).unwrap();
        let q = s.saturating_sub(back);
        if leaf_preds[s].iter().any(|l| l.predecessor == leaves[q]) {
            continue;
        }
        let kind = if next() % 4 == 0 {
            LinkType::StartToStart
        } else {
            LinkType::FinishToStart
        };
        leaf_preds[s].push(Link {
            predecessor: leaves[q],
            kind,
            lag: Lag::ZERO,
        });
        links += 1;
    }
    let mut z = Vec::new();
    for i in 0..DEPTH {
        uid += 1;
        let mut t = Task::new(TaskUid::new(uid), format!("Z{i}"), &options);
        t.duration = day(2);
        z.push(t);
    }
    let n = spine.len();
    for (i, s) in spine.iter().enumerate() {
        let hour = Lag::new(
            TimeBasis::Working,
            LagValue::Time {
                minutes: Minutes::new(60),
                unit: DurationUnit::Hour,
            },
        )
        .unwrap();
        z[i].predecessors.push(Link {
            predecessor: *s,
            kind: LinkType::FinishToStart,
            lag: hour,
        });
        z[(i + 1) % n].predecessors.push(Link {
            predecessor: *s,
            kind: LinkType::StartToStart,
            lag: Lag::ZERO,
        });
        z[(i + 2) % n].predecessors.push(Link {
            predecessor: *s,
            kind: LinkType::FinishToFinish,
            lag: Lag::ZERO,
        });
    }
    let mut leaf_preds = leaf_preds.into_iter();
    for mut t in tasks {
        if t.outline_level.get() > 1 && !spine.contains(&t.uid()) {
            t.predecessors = leaf_preds.next().unwrap_or_default();
        }
        p.tasks.push(t).unwrap();
    }
    for t in z {
        p.tasks.push(t).unwrap();
    }
    p
}

/// La hiérarchie hostile a la forme annoncée, se calcule sans erreur, et
/// les deux implémentations des marges et des % achevés (lot L15) y sont
/// égales.
#[test]
fn deep_hierarchy_shape() {
    let p = deep_hierarchy();
    assert_eq!(p.tasks.len(), 5_000);
    let links: usize = p.tasks.iter().map(|t| t.predecessors.len()).sum();
    assert_eq!(links, DEEP_LINKS);
    assert_eq!(p.tasks.iter().map(|t| t.outline_level.get()).max(), Some(257));
    let s = schedule(&p);
    let errors: Vec<&Diagnostic> = s
        .diagnostics
        .iter()
        .filter(|d| d.severity() == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    let summaries = s
        .tasks
        .values()
        .filter(|o| matches!(o, TaskOutcome::Scheduled(ts) if ts.role == TaskRole::Summary && !ts.degraded))
        .count();
    assert_eq!(summaries, 256);
    assert_eq!(s, schedule_reference(&p));
}

#[test]
#[ignore = "budget mesuré en release, job CI perf"]
fn perf_deep_hierarchy_256_5000() {
    let p = deep_hierarchy();
    // Référence (J3), informative : le coût que la stratégie rapide évite.
    let t = Instant::now();
    black_box(schedule_reference(black_box(&p)));
    eprintln!("schedule_reference (hiérarchie profonde) : {:?}", t.elapsed());
    budget_of(
        "hiérarchie de 256 niveaux, 5 000 tâches",
        &p,
        Duration::from_millis(30),
    );
}
