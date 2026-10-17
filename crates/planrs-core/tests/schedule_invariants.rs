//! Invariants du moteur sur des réseaux aléatoires (contrôle de la première
//! moitié du CPM, ADR-0009 ; les propriétés P13 à P17 de la spécification J2
//! viennent avec leur propre fichier).
//!
//! Générateur déterministe (graine tirée par proptest, xorshift) : 1 à 30
//! tâches, durées 0 à 4 800 (dont jalons et durées écoulées), hiérarchie
//! valide une fois sur deux, 0 à 3 prédécesseurs des quatre types (cycles et
//! doublons possibles), décalages ouvrés, écoulés ou en pourcentage, positifs
//! ou négatifs, calendriers Standard, 24 heures et Équipe de nuit (projet et
//! tâches), tâches inactives. Aucune contrainte, aucune tâche manuelle.
//!
//! Contrôles, sur toute tâche planifiée non dégradée :
//! - (a) chaque lien effectif entre feuilles actives est respecté (FS en
//!   `Moment` brut, SS/FF/SF en temps ouvré de l'échelle du successeur) ;
//!   une borne posée sur une récapitulative (FS/SS) l'est par toutes ses
//!   sous-tâches ;
//! - (b) `TF ≥ FS ≥ 0` ;
//! - (c) `ES ≤ EF`, `LS ≤ LF` ; une récapitulative couvre exactement ses
//!   enfants actifs planifiés ;
//! - (d) déterminisme : clone, aller-retour JSON du projet et du planning ;
//! - (e) point fixe : les feuilles passées en manuelles à leurs dates au plus
//!   tôt redonnent les mêmes dates et marges ;
//! - (f) `order` est topologique pour les liens effectifs entre feuilles ;
//! - une entrée par tâche, diagnostics triés et sans doublon.
//!
//! Chaos (aucune panique, une entrée par tâche, diagnostics triés) : dates
//! près de 1984 et de 2149, décalages géants, calendrier du projet sans temps
//! ouvré, calendriers inconnus, lignes vides, prédécesseurs inconnus,
//! manuelles aux dates quelconques, planification à rebours. Profondeur :
//! chaîne de 50 000 tâches, plan de 2 000 niveaux, 10 000 liens vers une
//! tâche, sans débordement de pile.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
)]

use std::collections::{BTreeMap, BTreeSet};

use planrs_core::calendar::{BuiltinCalendarNames, CalendarSet, TimeScale};
use planrs_core::{
    CalendarUid, Diagnostic, Duration, DurationUnit, Lag, LagValue, Link, LinkType, Minutes, Moment,
    OutlineLevel, Project, ProjectInit, Schedule, ScheduleAnchor, SchedulingMode, SignedPercent, Task,
    TaskOutcome, TaskRole, TaskUid, TimeBasis, percent_of, schedule,
};
use proptest::prelude::*;

/// xorshift64*.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Entier dans `[lo, hi]`.
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        let span = u64::try_from(hi - lo + 1).unwrap();
        lo + i64::try_from(self.next() % span).unwrap()
    }

    /// Vrai avec une probabilité de `pct` %.
    fn chance(&mut self, pct: u64) -> bool {
        self.next() % 100 < pct
    }
}

fn generate(seed: u64) -> Project {
    let mut r = Rng(seed | 1);
    let base: Moment = "2026-01-05T08:00".parse().unwrap();
    let mut offset = r.range(0, 300) * 1_440;
    if r.chance(30) {
        offset += r.range(0, 1_439);
    }
    let start = base.checked_add(Minutes::new(offset)).unwrap();
    let mut p = Project::new(ProjectInit {
        name: "Aléatoire".to_owned(),
        anchor: ScheduleAnchor::FromStart(start),
        current_date: start,
        calendar_names: BuiltinCalendarNames::default(),
    });
    if r.chance(25) {
        p.calendar = CalendarUid::new(u32::try_from(r.range(2, 3)).unwrap());
    }
    p.options.critical_slack_limit_days = u32::try_from(r.range(0, 1)).unwrap();
    p.options.multiple_critical_paths = r.chance(20);
    let n = r.range(1, 30);
    let hierarchical = r.chance(50);
    let mut prev_level: i64 = 1;
    let options = p.options;
    for i in 1..=n {
        let uid = TaskUid::new(u32::try_from(i).unwrap());
        let mut t = Task::new(uid, format!("T{i}"), &options);
        let minutes = if r.chance(10) { 0 } else { r.range(1, 4_800) };
        let basis = if r.chance(10) {
            TimeBasis::Elapsed
        } else {
            TimeBasis::Working
        };
        t.duration = Duration::new(Minutes::new(minutes), DurationUnit::Day, basis, false).unwrap();
        if hierarchical {
            let level = if i == 1 { 1 } else { r.range(1, prev_level + 1) };
            t.outline_level = OutlineLevel::new(u16::try_from(level).unwrap()).unwrap();
            prev_level = level;
        }
        if r.chance(20) {
            t.calendar = Some(CalendarUid::new(u32::try_from(r.range(1, 3)).unwrap()));
        }
        t.active = !r.chance(5);
        p.tasks.push(t).unwrap();
    }
    for i in 1..=n {
        let k = r.range(0, 3);
        for _ in 0..k {
            let pred = TaskUid::new(u32::try_from(r.range(1, n)).unwrap());
            let kind = match r.range(0, 9) {
                0..=5 => LinkType::FinishToStart,
                6 => LinkType::StartToStart,
                7 => LinkType::FinishToFinish,
                _ => LinkType::StartToFinish,
            };
            let lag = if r.chance(50) {
                Lag::ZERO
            } else if r.chance(10) {
                let basis = if r.chance(30) {
                    TimeBasis::Elapsed
                } else {
                    TimeBasis::Working
                };
                let percent = SignedPercent::new(r.range(-10_000, 10_000)).unwrap();
                Lag::new(basis, LagValue::Percent { percent }).unwrap()
            } else {
                let basis = if r.chance(15) {
                    TimeBasis::Elapsed
                } else {
                    TimeBasis::Working
                };
                let minutes = Minutes::new(r.range(-2_000, 2_000));
                Lag::new(
                    basis,
                    LagValue::Time {
                        minutes,
                        unit: DurationUnit::Day,
                    },
                )
                .unwrap()
            };
            let succ = p.tasks.get_mut(TaskUid::new(u32::try_from(i).unwrap())).unwrap();
            succ.predecessors.push(Link {
                predecessor: pred,
                kind,
                lag,
            });
        }
    }
    p
}

/// Parents (positions) recalculés à part (R-052).
fn parents(p: &Project) -> Vec<Option<usize>> {
    let mut stack: Vec<(u16, usize)> = Vec::new();
    let mut out = Vec::new();
    for (i, t) in p.tasks.iter().enumerate() {
        if t.is_null {
            out.push(None);
            continue;
        }
        let l = t.outline_level.get();
        while stack.last().is_some_and(|(sl, _)| *sl >= l) {
            stack.pop();
        }
        out.push(stack.last().map(|(_, pos)| *pos));
        stack.push((l, i));
    }
    out
}

fn is_ancestor(parents: &[Option<usize>], anc: usize, mut q: usize) -> bool {
    while let Some(par) = parents[q] {
        if par == anc {
            return true;
        }
        q = par;
    }
    false
}

fn check(p: &Project) {
    let s = schedule(p);
    // (d) Déterminisme.
    assert_eq!(s, schedule(&p.clone()));
    let pj = serde_json::to_string(p).unwrap();
    let p2: Project = serde_json::from_str(&pj).unwrap();
    assert_eq!(schedule(&p2), s);
    let sj = serde_json::to_string(&s).unwrap();
    assert_eq!(serde_json::from_str::<Schedule>(&sj).unwrap(), s);
    // Une entrée par tâche ; diagnostics triés, sans doublon.
    assert_eq!(s.tasks.len(), p.tasks.len());
    let mut sorted = s.diagnostics.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted, s.diagnostics);

    let (set, _) = CalendarSet::compile(p);
    let project_cal = set.get(p.calendar).unwrap();
    let cal_of = |t: &Task| t.calendar.map_or(project_cal, |c| set.get(c).unwrap());
    let place_of = |t: &Task| {
        if t.duration.basis() == TimeBasis::Elapsed {
            TimeScale::Elapsed
        } else {
            TimeScale::Working(cal_of(t))
        }
    };
    let par = parents(p);
    let cut: BTreeSet<(TaskUid, TaskUid)> = s
        .diagnostics
        .iter()
        .filter_map(|d| match d {
            Diagnostic::LinkIgnoredInCycle {
                predecessor,
                successor,
            } => Some((*predecessor, *successor)),
            _ => None,
        })
        .collect();
    let scheduled = |uid: TaskUid| match s.get(uid) {
        Some(TaskOutcome::Scheduled(ts)) if !ts.degraded => Some(*ts),
        _ => None,
    };
    let order_pos: BTreeMap<TaskUid, usize> = s.order.iter().enumerate().map(|(i, u)| (*u, i)).collect();

    // (b), (c).
    for (i, t) in p.tasks.iter().enumerate() {
        let Some(ts) = scheduled(t.uid()) else { continue };
        assert!(ts.early.start <= ts.early.finish, "{}: ES > EF", t.uid());
        assert!(ts.late.start <= ts.late.finish, "{}: LS > LF", t.uid());
        assert!(ts.total_slack >= ts.free_slack, "{}: TF < FS {ts:?}", t.uid());
        assert!(ts.free_slack >= Minutes::ZERO, "{}: FS < 0 {ts:?}", t.uid());
        if ts.role == TaskRole::Summary {
            let mut span: Option<(Moment, Moment)> = None;
            for (q, c) in p.tasks.iter().enumerate() {
                if par[q] != Some(i) {
                    continue;
                }
                if let Some(TaskOutcome::Scheduled(cs)) = s.get(c.uid()) {
                    span = Some(span.map_or((cs.early.start, cs.early.finish), |(a, b)| {
                        (a.min(cs.early.start), b.max(cs.early.finish))
                    }));
                }
            }
            assert_eq!(span, Some((ts.early.start, ts.early.finish)), "{}", t.uid());
        }
    }

    // (a), (f) : liens effectifs.
    for (si, st) in p.tasks.iter().enumerate() {
        let mut seen = BTreeSet::new();
        for l in &st.predecessors {
            if !seen.insert(l.predecessor) || l.predecessor == st.uid() {
                continue;
            }
            let Some(pi) = p.tasks.position(l.predecessor) else {
                continue;
            };
            let pt = p.tasks.at(pi).unwrap();
            if cut.contains(&(pt.uid(), st.uid()))
                || is_ancestor(&par, pi, si)
                || is_ancestor(&par, si, pi)
                || !pt.active
                || par.iter().enumerate().any(|(q, pp)| *pp == Some(pi) && q > pi)
            {
                continue;
            }
            let Some(pe) = scheduled(pt.uid()) else { continue };
            let lag = match l.lag.value {
                LagValue::Time { minutes, .. } => minutes,
                LagValue::Percent { percent } => percent_of(pt.duration.minutes(), percent).unwrap(),
            };
            let sigma = if l.lag.basis == TimeBasis::Elapsed {
                TimeScale::Elapsed
            } else {
                TimeScale::Working(cal_of(st))
            };
            let anchor = match l.kind {
                LinkType::FinishToStart | LinkType::FinishToFinish => pe.early.finish,
                LinkType::StartToStart | LinkType::StartToFinish => pe.early.start,
            };
            let Ok(bound) = (if lag == Minutes::ZERO {
                Ok(anchor)
            } else {
                sigma.add(anchor, lag)
            }) else {
                continue;
            };
            // Successeur feuille, ou sous-tâches d'une récapitulative (FS/SS).
            let targets: Vec<usize> = if par.contains(&Some(si)) {
                if matches!(l.kind, LinkType::FinishToFinish | LinkType::StartToFinish) {
                    continue;
                }
                (0..p.tasks.len())
                    .filter(|q| is_ancestor(&par, si, *q) && !par.contains(&Some(*q)))
                    .collect()
            } else {
                vec![si]
            };
            for q in targets {
                let qt = p.tasks.at(q).unwrap();
                let Some(qe) = scheduled(qt.uid()) else { continue };
                let place = place_of(qt);
                match l.kind {
                    LinkType::FinishToStart => assert!(
                        qe.early.start >= bound,
                        "FS {} → {}: {:?} < {bound}",
                        pt.uid(),
                        qt.uid(),
                        qe.early.start
                    ),
                    LinkType::StartToStart => assert!(
                        place.between(bound, qe.early.start).unwrap() >= Minutes::ZERO,
                        "SS {} → {}",
                        pt.uid(),
                        qt.uid()
                    ),
                    LinkType::FinishToFinish | LinkType::StartToFinish => assert!(
                        place.between(bound, qe.early.finish).unwrap() >= Minutes::ZERO,
                        "FF/SF {} → {}",
                        pt.uid(),
                        qt.uid()
                    ),
                }
                if qt.uid() == st.uid()
                    && let (Some(a), Some(b)) = (order_pos.get(&pt.uid()), order_pos.get(&qt.uid()))
                {
                    assert!(a < b, "ordre : {} avant {}", pt.uid(), qt.uid());
                }
            }
        }
    }

    // (e) Point fixe : feuilles actives automatiques → manuelles à leurs
    // dates au plus tôt.
    let mut fixed = p.clone();
    let uids: Vec<TaskUid> = fixed.tasks.iter().map(Task::uid).collect();
    for uid in uids {
        let t = fixed.tasks.get_mut(uid).unwrap();
        if let Some(TaskOutcome::Scheduled(ts)) = s.get(t.uid())
            && ts.role == TaskRole::Leaf
            && t.active
        {
            t.mode = SchedulingMode::Manual {
                start: Some(ts.early.start),
                finish: Some(ts.early.finish),
                duration: None,
            };
        }
    }
    let f = schedule(&fixed);
    for (uid, o) in &s.tasks {
        if let (TaskOutcome::Scheduled(a), Some(TaskOutcome::Scheduled(b))) = (o, f.tasks.get(uid)) {
            assert_eq!(a.early, b.early, "point fixe {uid}");
            assert_eq!(a.late, b.late, "point fixe {uid}");
            assert_eq!(a.total_slack, b.total_slack, "point fixe {uid}");
            assert_eq!(a.free_slack, b.free_slack, "point fixe {uid}");
        }
    }
    assert!(
        !f.diagnostics
            .iter()
            .any(|d| d.code() == "link.manual_not_honored"),
        "point fixe : {:?}",
        f.diagnostics
    );
}

/// Réseau chaotique : tout ce qui peut mal tourner.
fn chaos(seed: u64) -> Project {
    let mut p = generate(seed);
    let mut r = Rng(seed.rotate_left(17) | 1);
    let start = match r.range(0, 3) {
        0 => "2149-12-20T08:00",
        1 => "1984-01-02T08:00",
        _ => "2026-10-19T08:00",
    };
    let start: Moment = start.parse().unwrap();
    p.anchor = if r.chance(5) {
        ScheduleAnchor::FromFinish(start)
    } else {
        ScheduleAnchor::FromStart(start)
    };
    if r.chance(10) {
        let empty = planrs_core::calendar::Calendar::new_base(
            CalendarUid::new(9),
            "Vide".to_owned(),
            planrs_core::Week::from_fn(|_| planrs_core::calendar::DaySetting::NonWorking),
        );
        p.calendars.push(empty).unwrap();
        p.calendar = CalendarUid::new(9);
    }
    let uids: Vec<TaskUid> = p.tasks.iter().map(Task::uid).collect();
    let n = i64::try_from(uids.len()).unwrap();
    for uid in uids {
        let t = p.tasks.get_mut(uid).unwrap();
        if r.chance(5) {
            t.is_null = true;
        }
        if r.chance(5) {
            t.calendar = Some(CalendarUid::new(u32::try_from(r.range(4, 9)).unwrap()));
        }
        if r.chance(10) {
            let entered_start = start.checked_add(Minutes::new(r.range(-20_000, 20_000))).ok();
            let entered_finish = start.checked_add(Minutes::new(r.range(-20_000, 20_000))).ok();
            t.mode = SchedulingMode::Manual {
                start: if r.chance(70) { entered_start } else { None },
                finish: if r.chance(70) { entered_finish } else { None },
                duration: None,
            };
        }
        if r.chance(10) {
            t.predecessors.push(Link {
                predecessor: TaskUid::new(u32::try_from(n + r.range(1, 5)).unwrap()),
                kind: LinkType::FinishToStart,
                lag: Lag::ZERO,
            });
        }
        for l in &mut t.predecessors {
            if r.chance(10) {
                let minutes = Minutes::new(r.range(-87_308_640, 87_308_640));
                l.lag = Lag::new(
                    TimeBasis::Working,
                    LagValue::Time {
                        minutes,
                        unit: DurationUnit::Day,
                    },
                )
                .unwrap();
            }
        }
    }
    p
}

fn check_chaos(p: &Project) {
    let s = schedule(p);
    assert_eq!(s.tasks.len(), p.tasks.len());
    let mut sorted = s.diagnostics.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted, s.diagnostics);
    assert_eq!(s, schedule(&p.clone()));
    for (uid, o) in &s.tasks {
        if let TaskOutcome::Scheduled(ts) = o {
            assert!(ts.early.start <= ts.early.finish, "{uid}");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1_000))]

    #[test]
    fn schedule_survives_chaos(seed in any::<u64>()) {
        check_chaos(&chaos(seed));
    }
}

/// Chaîne FS de `n` tâches d'un jour.
fn chain(n: u32, start: &str) -> Project {
    let start: Moment = start.parse().unwrap();
    let mut p = Project::new(ProjectInit {
        name: "Chaîne".to_owned(),
        anchor: ScheduleAnchor::FromStart(start),
        current_date: start,
        calendar_names: BuiltinCalendarNames::default(),
    });
    let options = p.options;
    for i in 1..=n {
        let mut t = Task::new(TaskUid::new(i), String::new(), &options);
        if i > 1 {
            t.predecessors.push(Link::finish_to_start(TaskUid::new(i - 1)));
        }
        p.tasks.push(t).unwrap();
    }
    p
}

#[test]
fn deep_chain_of_50_000_tasks() {
    // 50 000 jours ouvrés dépassent 2149 : la fin de chaîne est non
    // planifiable et dégradée, sans panique ni récursion.
    let s = schedule(&chain(50_000, "1984-01-02T08:00"));
    assert_eq!(s.tasks.len(), 50_000);
    assert!(s.diagnostics.iter().any(|d| d.code() == "task.unschedulable"));
    let first = s.scheduled(TaskUid::new(1)).unwrap();
    assert_eq!(first.total_slack, Minutes::ZERO);
}

#[test]
fn deep_outline_of_2_000_levels() {
    let mut p = chain(2_000, "2026-10-19T08:00");
    let uids: Vec<TaskUid> = p.tasks.iter().map(Task::uid).collect();
    for (i, uid) in uids.iter().enumerate() {
        let t = p.tasks.get_mut(*uid).unwrap();
        t.predecessors.clear();
        t.outline_level = OutlineLevel::new(u16::try_from(i + 1).unwrap()).unwrap();
    }
    // Lien de la dernière (feuille) vers une tâche de premier niveau.
    let options = p.options;
    let mut last = Task::new(TaskUid::new(5_000), String::new(), &options);
    last.predecessors.push(Link::finish_to_start(TaskUid::new(1)));
    p.tasks.push(last).unwrap();
    let s = schedule(&p);
    let top = s.scheduled(TaskUid::new(1)).unwrap();
    assert_eq!(top.role, TaskRole::Summary);
    assert_eq!(top.early, s.scheduled(TaskUid::new(2_000)).unwrap().early);
}

#[test]
fn ten_thousand_links_into_one_task() {
    let mut p = chain(10_001, "2026-10-19T08:00");
    let uids: Vec<TaskUid> = p.tasks.iter().map(Task::uid).collect();
    for uid in &uids {
        p.tasks.get_mut(*uid).unwrap().predecessors.clear();
    }
    let target = p.tasks.get_mut(TaskUid::new(10_001)).unwrap();
    for i in (1..=10_000).rev() {
        target.predecessors.push(Link::finish_to_start(TaskUid::new(i)));
    }
    let s = schedule(&p);
    let t = s.scheduled(TaskUid::new(10_001)).unwrap();
    assert_eq!(t.early.start, "2026-10-20T08:00".parse::<Moment>().unwrap());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2_000))]

    #[test]
    fn schedule_invariants(seed in any::<u64>()) {
        check(&generate(seed));
    }
}

#[test]
fn schedule_invariants_fixed_seeds() {
    for seed in 0..200 {
        check(&generate(seed));
    }
}
