//! Utilisation des ressources par période (ADR-0013 D6 ; règle R-188, lot
//! L5). Contrat J7 (ADR-0015 D7, lot L0) : [`usage_with`] sur des
//! calendriers déjà compilés, granularité [`Granularity::Quarter`] et
//! utilisation par affectation ([`usage_by_assignment`]) ; trimestre et
//! utilisation par affectation sont des squelettes jusqu'au lot L4 (R-245,
//! R-246).
//!
//! Calculée à la demande depuis un [`Schedule`] (jamais stockée) : pour
//! chaque ressource demandée et chaque période civile qui coupe `[from,
//! to)`, travail planifié dans la période (bornée à `[from, to)`) et
//! surutilisation. Bornée (règle 10) : [`MAX_PERIODS`] périodes,
//! [`MAX_RESOURCES`] ressources.
//!
//! Travail d'une affectation dans `[a, b)` : W × (temps ouvré de son
//! échelle effective dans `[a, b)` ∩ span) ÷ (temps ouvré du span), soit le
//! débit effectif × les minutes ouvrées (R-186), calculé par différence de
//! cumuls arrondis : la somme des périodes qui couvrent le span vaut W
//! exactement.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, Months, NaiveDate};

use crate::calendar::{CalendarSet, Span, TimeScale};
use crate::duration::Work;
use crate::fixed::round_div;
use crate::ids::{AssignmentUid, ResourceUid, TaskUid};
use crate::model::{Project, ResourceKind};
use crate::schedule::load::{Scales, project_place};
use crate::schedule::{AssignmentSchedule, Schedule};
use crate::time::{Day, Moment, TimeOfDay};

/// Périodes d'un rapport au plus.
pub const MAX_PERIODS: usize = 1_000;
/// Ressources d'un rapport au plus.
pub const MAX_RESOURCES: usize = 1_000;
/// Lignes d'affectation d'un rapport par affectation au plus (R-245).
pub const MAX_ASSIGNMENT_ROWS: usize = 10_000;

/// Granularité des périodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Granularity {
    /// Jour civil.
    Day,
    /// Semaine (premier jour : `WeekStartDay` du projet).
    Week,
    /// Mois civil.
    Month,
    /// Trimestre civil (1er janvier, avril, juillet, octobre ; R-246).
    /// Squelette J7 L0 : aucune période jusqu'au lot L4.
    Quarter,
}

/// Rapport d'utilisation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageReport {
    /// Début de chaque période, dans l'ordre.
    pub periods: Vec<Moment>,
    /// Une ligne par ressource demandée, dans l'ordre demandé.
    pub rows: Vec<UsageRow>,
    /// Vrai si le rapport est coupé.
    pub truncated: bool,
}

/// Ligne d'une ressource : une valeur par période.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageRow {
    /// Ressource.
    pub resource: ResourceUid,
    /// Travail planifié dans chaque période.
    pub work: Vec<Work>,
    /// Surutilisation dans chaque période.
    pub overallocated: Vec<bool>,
}

/// Ligne d'une affectation (R-245) : travail par période, calculé comme la
/// ligne de sa ressource (R-188).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssignmentUsageRow {
    /// Affectation.
    pub assignment: AssignmentUid,
    /// Tâche affectée.
    pub task: TaskUid,
    /// Ressource affectée.
    pub resource: ResourceUid,
    /// Travail planifié dans chaque période.
    pub work: Vec<Work>,
}

/// Rapport d'utilisation par affectation (R-245).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssignmentUsageReport {
    /// Périodes, dans l'ordre (bornées à `[from, to)`).
    pub periods: Vec<Span>,
    /// Lignes triées par (ressource, tâche, UID d'affectation).
    pub rows: Vec<AssignmentUsageRow>,
    /// Total de chaque tâche affectée, période par période (Σ de ses
    /// affectations), dans l'ordre des UID.
    pub task_totals: Vec<(TaskUid, Vec<Work>)>,
}

/// Rapport refusé.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UsageError {
    /// Trop de périodes ou de ressources.
    #[error("rapport trop grand : au plus {periods} périodes et {resources} ressources")]
    Limit {
        /// Périodes au plus.
        periods: usize,
        /// Ressources au plus.
        resources: usize,
    },
}

impl UsageError {
    /// Code stable.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Limit { .. } => "usage.limit",
        }
    }
}

/// Utilisation des ressources `resources` sur `[from, to)`, par périodes
/// `by` (R-188). Lignes dans l'ordre demandé ; ressource inconnue ou de
/// coût : travail nul ; ressource matérielle : quantités (R-185) ;
/// affectation fictive (sans ressource) jamais comptée. `to <= from` :
/// aucune période. `truncated` : la surutilisation d'une ressource demandée
/// est coupée ([`crate::schedule::MAX_OVERLOADS`]).
///
/// # Errors
///
/// [`UsageError::Limit`] au-delà de [`MAX_PERIODS`] périodes ou de
/// [`MAX_RESOURCES`] ressources.
pub fn usage(
    p: &Project,
    s: &Schedule,
    from: Moment,
    to: Moment,
    by: Granularity,
    resources: &[ResourceUid],
) -> Result<UsageReport, UsageError> {
    let (set, _) = CalendarSet::compile(p);
    usage_with(p, s, &set, from, to, by, resources)
}

/// [`usage`] sur les calendriers déjà compilés `set`
/// (`CalendarSet::compile(p)`) : l'interface les compile une fois par
/// génération (ADR-0015 D7). Même résultat que [`usage`].
///
/// # Errors
///
/// Comme [`usage`].
pub fn usage_with(
    p: &Project,
    s: &Schedule,
    set: &CalendarSet,
    from: Moment,
    to: Moment,
    by: Granularity,
    resources: &[ResourceUid],
) -> Result<UsageReport, UsageError> {
    let limit = UsageError::Limit {
        periods: MAX_PERIODS,
        resources: MAX_RESOURCES,
    };
    if resources.len() > MAX_RESOURCES {
        return Err(limit);
    }
    let periods = periods(p, (from, to), by).ok_or(limit)?;
    // Bornes des périodes, coupées à `[from, to)` : n + 1 instants.
    let mut bounds: Vec<Moment> = Vec::with_capacity(periods.len().saturating_add(1));
    if !periods.is_empty() {
        bounds.push(from);
        bounds.extend(periods.iter().skip(1).copied());
        bounds.push(to);
    }
    let place = project_place(p, set);
    let mut scales = place.as_deref().map(|place| Scales::new(set, place));
    // Affectations des ressources demandées, dans l'ordre des UID.
    let wanted: BTreeSet<ResourceUid> = resources.iter().copied().collect();
    let mut by_resource: BTreeMap<ResourceUid, Vec<&AssignmentSchedule>> = BTreeMap::new();
    for a in s.assignments.values() {
        let Some(r) = a.resource.filter(|r| wanted.contains(r)) else {
            continue;
        };
        if let Some(sc) = scales.as_mut() {
            sc.prepare(a.calendar);
        }
        by_resource.entry(r).or_default().push(a);
    }
    let mut truncated = false;
    let mut rows = Vec::with_capacity(resources.len());
    for &r in resources {
        let n = periods.len();
        let mut work: Vec<i128> = vec![0; n];
        let counted = p
            .resources
            .get(r)
            .is_some_and(|res| res.kind != ResourceKind::Cost);
        if let (true, Some(sc), Some(list)) = (counted, scales.as_ref(), by_resource.get(&r)) {
            for a in list {
                if let Some(scale) = sc.get(a.calendar) {
                    spread(a, scale, &bounds, &mut work);
                }
            }
        }
        let mut overallocated = vec![false; n];
        if let Some(load) = s.resources.get(&r) {
            truncated |= load.truncated;
            for o in &load.overloads {
                mark(&bounds, (o.from, o.to), &mut overallocated);
            }
        }
        rows.push(UsageRow {
            resource: r,
            work: work.into_iter().map(clamp_work).collect(),
            overallocated,
        });
    }
    Ok(UsageReport {
        periods,
        rows,
        truncated,
    })
}

/// Utilisation par affectation des ressources `resources` sur `[from, to)`
/// (R-245) : une ligne par affectation de travail, totaux par tâche.
///
/// Squelette J7 L0 : périodes et lignes vides (lot L4) ; seules les bornes
/// sont contrôlées.
///
/// # Errors
///
/// [`UsageError::Limit`] au-delà de [`MAX_PERIODS`] périodes, de
/// [`MAX_RESOURCES`] ressources ou de [`MAX_ASSIGNMENT_ROWS`] lignes.
pub fn usage_by_assignment(
    p: &Project,
    s: &Schedule,
    set: &CalendarSet,
    from: Moment,
    to: Moment,
    by: Granularity,
    resources: &[ResourceUid],
) -> Result<AssignmentUsageReport, UsageError> {
    let _ = (s, set);
    let limit = UsageError::Limit {
        periods: MAX_PERIODS,
        resources: MAX_RESOURCES,
    };
    if resources.len() > MAX_RESOURCES {
        return Err(limit);
    }
    periods(p, (from, to), by).ok_or(limit)?;
    Ok(AssignmentUsageReport {
        periods: Vec::new(),
        rows: Vec::new(),
        task_totals: Vec::new(),
    })
}

/// Débuts des périodes civiles `by` qui coupent `[from, to)` : la première
/// contient `from` (jour à 00:00, semaine au `week_start_day` du projet,
/// mois au 1er). `None` au-delà de [`MAX_PERIODS`].
fn periods(p: &Project, (from, to): (Moment, Moment), by: Granularity) -> Option<Vec<Moment>> {
    let mut out = Vec::new();
    // Trimestre : squelette J7 L0, aucune période jusqu'au lot L4 (R-246).
    if to <= from || by == Granularity::Quarter {
        return Some(out);
    }
    let mut day = from.day();
    match by {
        Granularity::Day | Granularity::Quarter => {}
        Granularity::Week => {
            let mut back: u8 = 0;
            while day.weekday() != p.options.week_start_day && back < 7 {
                let Ok(prev) = day.checked_add_days(-1) else { break };
                day = prev;
                back = back.saturating_add(1);
            }
        }
        Granularity::Month => {
            let d = day.to_naive();
            day = NaiveDate::from_ymd_opt(d.year(), d.month(), 1)
                .and_then(|first| Day::from_naive(first).ok())
                .unwrap_or(day);
        }
    }
    while let Ok(start) = Moment::at(day, TimeOfDay::MIDNIGHT) {
        if start >= to {
            break;
        }
        if out.len() >= MAX_PERIODS {
            return None;
        }
        out.push(start);
        let next = match by {
            Granularity::Day => day.checked_add_days(1).ok(),
            Granularity::Week => day.checked_add_days(7).ok(),
            Granularity::Month => day
                .to_naive()
                .checked_add_months(Months::new(1))
                .and_then(|d| Day::from_naive(d).ok()),
            Granularity::Quarter => None,
        };
        match next {
            Some(n) if n > day => day = n,
            _ => break,
        }
    }
    Some(out)
}

/// Ajoute à `work` le travail de l'affectation `a` dans chaque période
/// (`bounds` : n + 1 bornes), au prorata du temps ouvré de son échelle ;
/// span sans temps ouvré : tout le travail dans la période de son début.
fn spread(a: &AssignmentSchedule, scale: TimeScale<'_>, bounds: &[Moment], work: &mut [i128]) {
    let w = i128::from(a.work.minutes().get());
    if w == 0 {
        return;
    }
    let Span { start, finish } = a.span;
    let total = scale.between(start, finish).map_or(0, |m| i128::from(m.get()));
    // Première période dont la fin dépasse le début du span.
    let first = bounds.partition_point(|&b| b <= start).saturating_sub(1);
    if total <= 0 {
        let inside = bounds.first().is_some_and(|&b| b <= start) && bounds.last().is_some_and(|&b| start < b);
        if let Some(cell) = work.get_mut(first).filter(|_| inside) {
            *cell = cell.saturating_add(w);
        }
        return;
    }
    // Travail cumulé à l'instant `t` (borné au span).
    let cumul = |t: Moment| -> i128 {
        let t = t.clamp(start, finish);
        let m = scale.between(start, t).map_or(0, |m| i128::from(m.get()));
        w.checked_mul(m)
            .and_then(|num| round_div(num, total))
            .map_or(0, i128::from)
    };
    for (i, cell) in work.iter_mut().enumerate().skip(first) {
        let (Some(&lo), Some(&hi)) = (bounds.get(i), bounds.get(i.saturating_add(1))) else {
            break;
        };
        if lo >= finish {
            break;
        }
        if hi <= start {
            continue;
        }
        *cell = cell.saturating_add(cumul(hi).saturating_sub(cumul(lo)));
    }
}

/// Marque les périodes que l'intervalle `[from, to)` coupe.
fn mark(bounds: &[Moment], (from, to): (Moment, Moment), flags: &mut [bool]) {
    let first = bounds.partition_point(|&b| b <= from).saturating_sub(1);
    for (i, flag) in flags.iter_mut().enumerate().skip(first) {
        let (Some(&lo), Some(&hi)) = (bounds.get(i), bounds.get(i.saturating_add(1))) else {
            break;
        };
        if lo >= to {
            break;
        }
        if from < hi && to > lo {
            *flag = true;
        }
    }
}

/// Travail borné au domaine de [`Work`].
fn clamp_work(v: i128) -> Work {
    let m = v.clamp(0, i128::from(Work::MAX_MINUTES));
    i64::try_from(m)
        .ok()
        .and_then(|m| Work::new(crate::duration::Minutes::new(m)).ok())
        .unwrap_or(Work::ZERO)
}
