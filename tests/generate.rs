//! Générateurs déterministes de grands réseaux pour la mesure du moteur
//! (spécification J2 § 6) : sans dépendance d'aléa (xorshift à graine).

#![allow(
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::same_item_push,
    reason = "paquet de test : générateur sur des tailles fixes et petites, un débordement serait un bug du test ; plan écrit niveau par niveau"
)]

use std::collections::BTreeSet;

use planrs_core::calendar::{BuiltinCalendarNames, Calendar, CalendarException, DaySetting, Recurrence};
use planrs_core::{
    Assignment, AssignmentUid, CalendarUid, Constraint, Day, DayRange, Duration, DurationUnit, Lag, LagValue,
    Link, LinkType, Minutes, Moment, OutlineLevel, Project, ProjectInit, Resource, ResourceUid,
    ScheduleAnchor, SignedPercent, Task, TaskUid, TimeBasis, Units, Work,
};

/// xorshift64*, déterministe sur toutes les plateformes.
pub struct Rng(u64);

impl Rng {
    /// Générateur de graine `seed` (une graine nulle est remplacée).
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    /// Entier suivant.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Entier dans `[lo, hi]` (`lo ≤ hi`).
    ///
    /// # Panics
    ///
    /// Si `lo > hi`.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        let span = u64::try_from(hi - lo + 1).unwrap_or_else(|_| panic!("intervalle vide {lo}..={hi}"));
        lo + i64::try_from(self.next_u64() % span).unwrap_or(0)
    }

    /// Indice dans `[0, n)` (`n > 0`).
    pub fn index(&mut self, n: usize) -> usize {
        let n = u64::try_from(n).unwrap_or(1).max(1);
        usize::try_from(self.next_u64() % n).unwrap_or(0)
    }

    /// Vrai avec une probabilité de `pct` %.
    pub fn chance(&mut self, pct: u64) -> bool {
        self.next_u64() % 100 < pct
    }
}

/// Nombre de récapitulatives de premier niveau du grand réseau.
const TOP: usize = 25;
/// Sous-récapitulatives par récapitulative (niveaux 2 et 3).
const FAN: usize = 4;
/// Nombre total de tâches.
const TASKS: usize = 5_000;
/// Nombre de liens.
const LINKS: usize = 10_000;
/// Bruit de l'ordre caché et fenêtre des prédécesseurs : environ 20 % des
/// liens remontent la liste des ID.
const NOISE: i64 = 26;
const BEFORE: usize = 20;
const AFTER: usize = 26;

/// Grand réseau de mesure (spécification J2 § 6) : 5 000 tâches dont 525
/// récapitulatives sur 4 niveaux (25 × 4 × 4 récapitulatives, feuilles au
/// niveau 4) ; 10 000 liens entre feuilles (70 % FS, 10 % de chacun des
/// autres types) tirés dans un ordre topologique caché proche de l'ordre des
/// ID (environ 20 % remontent la liste : le chemin de Kahn est forcé) ;
/// 30 % décalés (ouvrés, écoulés, en %, négatifs) ; calendrier Standard avec
/// 100 jours fériés, 24 heures et Équipe de nuit (10 % des feuilles ont un
/// calendrier propre) ; 5 % de durées écoulées. Aucun lien entre une
/// récapitulative et sa descendante, aucun cycle (les liens ne touchent que
/// des feuilles, dans l'ordre caché).
///
/// # Panics
///
/// Jamais pour une graine quelconque (les valeurs sont toutes valides).
#[must_use]
pub fn large_network(seed: u64) -> Project {
    let mut r = Rng::new(seed);
    let start: Moment = "2026-01-05T08:00".parse().unwrap_or(Moment::MIN);
    let mut p = Project::new(ProjectInit {
        name: "Grand réseau".to_owned(),
        anchor: ScheduleAnchor::FromStart(start),
        current_date: start,
        calendar_names: BuiltinCalendarNames::default(),
    });
    add_holidays(&mut p, &mut r, 100);
    let options = p.options;
    // Plan : niveaux 1 à 3 récapitulatifs, feuilles au niveau 4.
    let summaries = TOP * (1 + FAN + FAN * FAN);
    let leaves_per_l3 = (TASKS - summaries) / (TOP * FAN * FAN);
    let mut extra = (TASKS - summaries) % (TOP * FAN * FAN);
    let mut levels: Vec<u16> = Vec::with_capacity(TASKS);
    for _ in 0..TOP {
        levels.push(1);
        for _ in 0..FAN {
            levels.push(2);
            for _ in 0..FAN {
                levels.push(3);
                let mut k = leaves_per_l3;
                if extra > 0 {
                    k += 1;
                    extra -= 1;
                }
                levels.extend(std::iter::repeat_n(4, k));
            }
        }
    }
    let mut leaves: Vec<usize> = Vec::new();
    for (i, level) in levels.iter().enumerate() {
        let uid = TaskUid::new(u32::try_from(i + 1).unwrap_or(u32::MAX));
        let mut t = Task::new(uid, format!("T{}", i + 1), &options);
        t.outline_level = OutlineLevel::new(*level).unwrap_or(OutlineLevel::TOP);
        if *level == 4 {
            let minutes = if r.chance(10) { 0 } else { r.range(60, 4_800) };
            let basis = if r.chance(5) {
                TimeBasis::Elapsed
            } else {
                TimeBasis::Working
            };
            t.duration =
                Duration::new(Minutes::new(minutes), DurationUnit::Day, basis, false).unwrap_or(t.duration);
            if r.chance(10) {
                t.calendar = Some(CalendarUid::new(if r.chance(50) { 2 } else { 3 }));
            }
            leaves.push(i);
        } else {
            t.duration = Duration::new(Minutes::ZERO, DurationUnit::Day, TimeBasis::Working, false)
                .unwrap_or(t.duration);
        }
        if let Err(e) = p.tasks.push(t) {
            panic!("tâche générée invalide : {e}");
        }
    }
    // Ordre caché : rang = indice de la feuille + bruit ; un lien va
    // toujours d'un rang plus petit vers un rang plus grand.
    let rank: Vec<i64> = (0..leaves.len())
        .map(|k| i64::try_from(k).unwrap_or(0) + r.range(0, NOISE))
        .collect();
    let mut pairs: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut links: Vec<Vec<Link>> = vec![Vec::new(); TASKS];
    while pairs.len() < LINKS {
        let s = r.index(leaves.len());
        let lo = s.saturating_sub(BEFORE);
        let p_k = lo + r.index(s - lo + AFTER + 1);
        if p_k >= leaves.len() || p_k == s || rank[p_k] >= rank[s] || !pairs.insert((p_k, s)) {
            continue;
        }
        let kind = match r.range(0, 9) {
            0..=6 => LinkType::FinishToStart,
            7 => LinkType::StartToStart,
            8 => LinkType::FinishToFinish,
            _ => LinkType::StartToFinish,
        };
        let lag = if r.chance(70) {
            Lag::ZERO
        } else {
            match r.range(0, 3) {
                0 => time_lag(TimeBasis::Working, r.range(-480, 960)),
                1 => time_lag(TimeBasis::Elapsed, r.range(-1_440, 2_880)),
                2 => percent_lag(r.range(-5_000, 10_000)),
                _ => time_lag(TimeBasis::Working, -r.range(1, 480)),
            }
        };
        links[leaves[s]].push(Link {
            predecessor: TaskUid::new(u32::try_from(leaves[p_k] + 1).unwrap_or(u32::MAX)),
            kind,
            lag,
        });
    }
    let uids: Vec<TaskUid> = p.tasks.iter().map(Task::uid).collect();
    for (uid, l) in uids.into_iter().zip(links) {
        if let Some(t) = p.tasks.get_mut(uid) {
            t.predecessors = l;
        }
    }
    p
}

fn time_lag(basis: TimeBasis, minutes: i64) -> Lag {
    Lag::new(
        basis,
        LagValue::Time {
            minutes: Minutes::new(minutes),
            unit: DurationUnit::Day,
        },
    )
    .unwrap_or(Lag::ZERO)
}

fn percent_lag(pct: i64) -> Lag {
    SignedPercent::new(pct)
        .ok()
        .and_then(|percent| Lag::new(TimeBasis::Working, LagValue::Percent { percent }).ok())
        .unwrap_or(Lag::ZERO)
}

/// `count` jours fériés (exceptions chômées d'un jour) au calendrier
/// Standard (UID 1), en 2026 et 2027.
fn add_holidays(p: &mut Project, r: &mut Rng, count: usize) {
    let Some(cal) = p.calendars.get_mut(CalendarUid::new(1)) else {
        return;
    };
    let first = "2026-01-01".parse::<Day>().map_or(0, Day::index);
    let mut added = 0;
    while added < count {
        let offset = i32::try_from(r.range(0, 729)).unwrap_or(0);
        let Ok(day) = Day::new(first + offset) else {
            continue;
        };
        let Ok(period) = DayRange::new(day, day) else {
            continue;
        };
        let e = CalendarException {
            name: format!("Férié {added}"),
            period,
            setting: DaySetting::NonWorking,
            recurrence: Recurrence::Period,
            occurrences: None,
        };
        if cal.add_exception(e).is_ok() {
            added += 1;
        }
    }
}

/// Grand réseau contraint (spécification J3 § 9.2, cas `5000_constrained`) :
/// le réseau de [`large_network`], plus, sur les feuilles, 20 % de
/// contraintes datées (six types, dates de −3 à +120 jours autour du début,
/// à la minute), 5 % d'ALAP et 5 % d'échéances (même plage) ; option
/// « respecter les contraintes » active.
///
/// # Panics
///
/// Jamais pour une graine quelconque (les valeurs sont toutes valides).
#[must_use]
pub fn constrained_network(seed: u64) -> Project {
    let mut p = large_network(seed);
    let ScheduleAnchor::FromStart(start) = p.anchor else {
        return p;
    };
    add_constraints(&mut p, seed, start, ScheduleAnchor::FromStart(start));
    p
}

/// Grand réseau affecté (ADR-0013, cas `5000_assigned`) : le réseau de
/// [`large_network`] dont chaque tâche reçoit deux affectations de travail
/// à 100 % (10 000 au total), travail = durée de la tâche ; 20 ressources,
/// une sur deux sur un calendrier propre dérivé du Standard ; une
/// affectation sur dix décalée d'un jour (`Delay` 480 min). Les feuilles
/// automatiques à deux calendriers effectifs sont placées par leurs parts
/// (R-173).
///
/// # Panics
///
/// Jamais pour une graine quelconque (les valeurs sont toutes valides).
#[must_use]
pub fn assigned_network(seed: u64) -> Project {
    let mut p = large_network(seed);
    for k in 1..=20u32 {
        let mut r = Resource::new(ResourceUid::new(k), format!("R{k}"));
        if k % 2 == 0 {
            let cal = CalendarUid::new(100 + k);
            if let Err(e) = p
                .calendars
                .push(Calendar::new_derived(cal, format!("R{k}"), CalendarUid::new(1)))
            {
                panic!("calendrier généré invalide : {e}");
            }
            r.calendar = Some(cal);
        }
        if let Err(e) = p.resources.push(r) {
            panic!("ressource générée invalide : {e}");
        }
    }
    let tasks: Vec<_> = p.tasks.iter().map(|t| (t.uid(), t.duration.minutes())).collect();
    let mut next = 1u32;
    for (i, (task, d)) in tasks.into_iter().enumerate() {
        for j in 0..2u32 {
            let k = u32::try_from(i % 20).unwrap_or(0) + 1;
            let res = ResourceUid::new((k + j * 7 - 1) % 20 + 1);
            let mut a = Assignment::new(AssignmentUid::new(next), task, Some(res));
            a.units = Units::FULL;
            a.work = Work::new(d).unwrap_or(Work::ZERO);
            if next.is_multiple_of(10) {
                a.delay = Minutes::new(480);
            }
            if let Err(e) = p.assignments.push(a) {
                panic!("affectation générée invalide : {e}");
            }
            next += 1;
        }
    }
    p
}

/// Jour non ouvré d'UID de calendrier `cal`, `offset` jours après le
/// 2026-01-01 ; faux si le jour est déjà pris.
fn add_leave(p: &mut Project, cal: CalendarUid, offset: i64, name: &str) -> bool {
    let first = "2026-01-01".parse::<Day>().map_or(0, Day::index);
    let Some(c) = p.calendars.get_mut(cal) else {
        return false;
    };
    let Ok(day) = Day::new(first + i32::try_from(offset).unwrap_or(0)) else {
        return false;
    };
    let Ok(period) = DayRange::new(day, day) else {
        return false;
    };
    c.add_exception(CalendarException {
        name: name.to_owned(),
        period,
        setting: DaySetting::NonWorking,
        recurrence: Recurrence::Period,
        occurrences: None,
    })
    .is_ok()
}

/// Projet hostile (relecture finale J5, adversaire F2) : 10 calendriers de
/// tâche et 100 calendriers de ressource dérivés du Standard, chacun chômé
/// un jour sur deux sur tout le domaine (environ 60 000 segments compilés)
/// et un jour propre ; 1 000 tâches d'un jour couvrent les 1 000 paires
/// (tâche, ressource). Sans budget des calendriers combinés, environ 4 Go
/// de segments.
///
/// # Panics
///
/// Jamais (valeurs valides).
#[must_use]
pub fn hostile_combined_calendars() -> Project {
    let start: Moment = "2026-10-19T08:00".parse().unwrap_or(Moment::MIN);
    let mut p = Project::new(ProjectInit {
        name: "Calendriers hostiles".to_owned(),
        anchor: ScheduleAnchor::FromStart(start),
        current_date: start,
        calendar_names: BuiltinCalendarNames::default(),
    });
    let (Ok(first), Ok(last)) = (Day::new(0), Day::new(60_630)) else {
        return p;
    };
    let Ok(domain) = DayRange::new(first, last) else {
        return p;
    };
    for k in 0..110u32 {
        let cal = CalendarUid::new(100 + k);
        let mut c = Calendar::new_derived(cal, format!("C{k}"), CalendarUid::new(1));
        let alternate = CalendarException {
            name: "Un jour sur deux".to_owned(),
            period: domain,
            setting: DaySetting::NonWorking,
            recurrence: Recurrence::Daily { every: 2 },
            occurrences: None,
        };
        if let Err(e) = c.add_exception(alternate) {
            panic!("exception générée invalide : {e}");
        }
        if let Err(e) = p.calendars.push(c) {
            panic!("calendrier généré invalide : {e}");
        }
        let _ = add_leave(&mut p, cal, 300 + 2 * i64::from(k) + 1, "Propre");
    }
    for k in 0..100u32 {
        let mut r = Resource::new(ResourceUid::new(k + 1), format!("R{k}"));
        r.calendar = Some(CalendarUid::new(110 + k));
        if let Err(e) = p.resources.push(r) {
            panic!("ressource générée invalide : {e}");
        }
    }
    let options = p.options;
    for i in 0..1_000u32 {
        let mut t = Task::new(TaskUid::new(i + 1), format!("T{i}"), &options);
        t.duration = Duration::new(Minutes::new(480), DurationUnit::Day, TimeBasis::Working, false)
            .unwrap_or_else(|e| panic!("durée générée invalide : {e}"));
        t.calendar = Some(CalendarUid::new(100 + i % 10));
        if let Err(e) = p.tasks.push(t) {
            panic!("tâche générée invalide : {e}");
        }
        let mut a = Assignment::new(
            AssignmentUid::new(i + 1),
            TaskUid::new(i + 1),
            Some(ResourceUid::new(i / 10 + 1)),
        );
        a.work = Work::new(Minutes::new(480)).unwrap_or(Work::ZERO);
        if let Err(e) = p.assignments.push(a) {
            panic!("affectation générée invalide : {e}");
        }
    }
    p
}

/// Projet hostile (relecture finale J5, adversaire F1) : une feuille d'un
/// jour, une ressource, `n` affectations de cette ressource à la tâche,
/// 8 h chacune, la k-ième décalée de k minutes (toutes des parts
/// distinctes).
///
/// # Panics
///
/// Jamais (valeurs valides).
#[must_use]
pub fn hostile_parts(n: u32) -> Project {
    let start: Moment = "2026-10-19T08:00".parse().unwrap_or(Moment::MIN);
    let mut p = Project::new(ProjectInit {
        name: "Parts hostiles".to_owned(),
        anchor: ScheduleAnchor::FromStart(start),
        current_date: start,
        calendar_names: BuiltinCalendarNames::default(),
    });
    let options = p.options;
    let mut t = Task::new(TaskUid::new(1), "A".to_owned(), &options);
    t.duration = Duration::new(Minutes::new(480), DurationUnit::Day, TimeBasis::Working, false)
        .unwrap_or_else(|e| panic!("durée générée invalide : {e}"));
    if let Err(e) = p.tasks.push(t) {
        panic!("tâche générée invalide : {e}");
    }
    if let Err(e) = p
        .resources
        .push(Resource::new(ResourceUid::new(1), "R".to_owned()))
    {
        panic!("ressource générée invalide : {e}");
    }
    for k in 0..n {
        let mut a = Assignment::new(
            AssignmentUid::new(k + 1),
            TaskUid::new(1),
            Some(ResourceUid::new(1)),
        );
        a.work = Work::new(Minutes::new(480)).unwrap_or(Work::ZERO);
        a.delay = Minutes::new(i64::from(k));
        if let Err(e) = p.assignments.push(a) {
            panic!("affectation générée invalide : {e}");
        }
    }
    p
}

/// Grand réseau planifié à rebours (spécification J3 § 9.2, cas
/// `5000_from_finish`) : le réseau de [`large_network`], ancré sur sa fin
/// calculée depuis le début, toutes ses tâches ALAP (défaut à rebours,
/// R-108), plus, sur les feuilles, 20 % de contraintes datées, 5 % d'ASAP et
/// 5 % d'échéances, dates de −120 à +3 jours autour de la fin.
///
/// # Panics
///
/// Jamais pour une graine quelconque (les valeurs sont toutes valides).
#[must_use]
pub fn from_finish_network(seed: u64) -> Project {
    let mut p = large_network(seed);
    let finish = planrs_core::schedule(&p).project_finish;
    let anchor = ScheduleAnchor::FromFinish(finish);
    p.anchor = anchor;
    add_constraints(&mut p, seed, finish, anchor);
    p
}

/// Contraintes du grand réseau : contrainte par défaut du sens sur toutes
/// les tâches (R-108), puis, sur les feuilles, 20 % de contraintes datées,
/// 5 % de la contrainte opposée (ALAP depuis le début, ASAP à rebours) et
/// 5 % d'échéances, dates à la minute autour de `around` (−3 à +120 jours
/// depuis le début, −120 à +3 jours à rebours).
fn add_constraints(p: &mut Project, seed: u64, around: Moment, anchor: ScheduleAnchor) {
    let mut r = Rng::new(seed.rotate_left(23));
    let (lo, hi, opposite) = match anchor {
        ScheduleAnchor::FromStart(_) => (-3 * 1_440, 120 * 1_440, Constraint::AsLateAsPossible),
        ScheduleAnchor::FromFinish(_) => (-120 * 1_440, 3 * 1_440, Constraint::AsSoonAsPossible),
    };
    let uids: Vec<TaskUid> = p.tasks.iter().map(Task::uid).collect();
    for uid in uids {
        let Some(t) = p.tasks.get_mut(uid) else {
            continue;
        };
        t.constraint = Constraint::default_for(&anchor);
        if t.outline_level.get() != 4 {
            continue;
        }
        let date = |r: &mut Rng| {
            around
                .checked_add(Minutes::new(r.range(lo, hi)))
                .unwrap_or(around)
        };
        if r.chance(20) {
            let d = date(&mut r);
            t.constraint = match r.range(0, 5) {
                0 => Constraint::MustStartOn { date: d },
                1 => Constraint::MustFinishOn { date: d },
                2 => Constraint::StartNoEarlierThan { date: d },
                3 => Constraint::StartNoLaterThan { date: d },
                4 => Constraint::FinishNoEarlierThan { date: d },
                _ => Constraint::FinishNoLaterThan { date: d },
            };
        } else if r.chance(6) {
            t.constraint = opposite;
        }
        if r.chance(5) {
            t.deadline = Some(date(&mut r));
        }
    }
}

/// Chaîne FS de `n` tâches d'un jour (mesure).
///
/// # Panics
///
/// Si `n` dépasse `u32::MAX`.
#[must_use]
pub fn chain(n: u32) -> Project {
    let start: Moment = "2026-01-05T08:00".parse().unwrap_or(Moment::MIN);
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
        if let Err(e) = p.tasks.push(t) {
            panic!("tâche générée invalide : {e}");
        }
    }
    p
}

/// `n` tâches indépendantes et une dernière qui les suit toutes (FS).
///
/// # Panics
///
/// Si `n + 1` dépasse `u32::MAX`.
#[must_use]
pub fn fan_in(n: u32) -> Project {
    let mut p = chain(n + 1);
    let uids: Vec<TaskUid> = p.tasks.iter().map(Task::uid).collect();
    for uid in &uids {
        if let Some(t) = p.tasks.get_mut(*uid) {
            t.predecessors.clear();
        }
    }
    if let Some(target) = p.tasks.get_mut(TaskUid::new(n + 1)) {
        for i in (1..=n).rev() {
            target.predecessors.push(Link::finish_to_start(TaskUid::new(i)));
        }
    }
    p
}
