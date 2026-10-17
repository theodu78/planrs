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

use planrs_core::calendar::{BuiltinCalendarNames, CalendarException, DaySetting, Recurrence};
use planrs_core::{
    CalendarUid, Day, DayRange, Duration, DurationUnit, Lag, LagValue, Link, LinkType, Minutes, Moment,
    OutlineLevel, Project, ProjectInit, ScheduleAnchor, SignedPercent, Task, TaskUid, TimeBasis,
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
