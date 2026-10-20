//! Tâches (R-120, R-127, R-129, R-132) : champs modélisés, liens, mode
//! manuel ; ID, numéro hiérarchique et drapeau `Summary` déduits.

use planrs_core::{
    Constraint, Duration, Lag, LagValue, Link, Minutes, Project, SchedulingMode, Subject, Task, TaskUid, Work,
};

use super::computed::{TaskValues, Values};
use super::emit::Emitter;
use super::{
    NO_EXTRAS, OrderProfile, Planned, WriteCode, baseline, entity, leveling, max_len, percent, plan, unscoped,
};
use crate::codes;
use crate::scalar::{format_bool, format_datetime, format_iso_duration};
use crate::schema::Parent;

/// Position d'une tâche dans le document.
struct Row<'p> {
    task: &'p Task,
    /// `ID` (0 pour la récapitulative du projet).
    id: u32,
    /// Récapitulative (R-052) ; UID 0 : vrai.
    summary: bool,
    /// `OutlineLevel` (0 pour l'UID 0).
    level: u16,
    /// `OutlineNumber`.
    outline_number: String,
    /// Valeurs calculées à écrire (R-122).
    values: Option<&'p TaskValues>,
}

/// Écrit la récapitulative du projet puis les tâches, dans l'ordre des ID.
pub(crate) fn write_all(em: &mut Emitter, p: &Project, values: Option<&Values>, profile: OrderProfile) {
    let of = |uid: TaskUid| values.and_then(|v| v.tasks.get(&uid));
    if let Some(summary) = &p.summary_task {
        let row = Row {
            task: summary,
            id: 0,
            summary: true,
            level: 0,
            outline_number: "0".to_owned(),
            values: of(TaskUid::PROJECT_SUMMARY),
        };
        write(em, &row, profile);
    }
    let flags = summary_flags(p);
    let numbers = outline_numbers(p);
    for ((position, task), (summary, number)) in
        p.tasks.iter().enumerate().zip(flags.into_iter().zip(numbers))
    {
        let id = u32::try_from(position)
            .ok()
            .and_then(|n| n.checked_add(1))
            .unwrap_or(u32::MAX);
        let row = Row {
            task,
            id,
            summary,
            level: task.outline_level.get(),
            outline_number: number,
            values: of(task.uid()),
        };
        write(em, &row, profile);
    }
}

/// R-052 : récapitulative ⇔ la tâche non vide suivante a un niveau
/// supérieur.
fn summary_flags(p: &Project) -> Vec<bool> {
    let mut out = vec![false; p.tasks.len()];
    let mut next: Option<u16> = None;
    for (slot, task) in out.iter_mut().zip(p.tasks.iter()).rev() {
        if task.is_null {
            continue;
        }
        let level = task.outline_level.get();
        *slot = next.is_some_and(|n| n > level);
        next = Some(level);
    }
    out
}

/// Numéros hiérarchiques (« 1 », « 1.2 »…) des tâches non vides ; une ligne
/// vide reçoit une chaîne vide sans faire avancer les compteurs.
///
/// R-120 : la profondeur d'une tâche est celle de la hiérarchie du moteur
/// (pile des niveaux, R-052 : parent = tâche non vide précédente de niveau
/// strictement inférieur), jamais son `OutlineLevel` brut : un niveau qui
/// saute (9 après 1) compte pour le précédent + 1, deux tâches au même
/// niveau sont sœurs. Règle 10 : sans cela, 60 000 tâches au niveau 65 535
/// écriraient chacune ~131 Ko. Un numéro est en outre construit au plus
/// jusqu'au `maxLength` du schéma + 1 caractère (l'émetteur le tronque et
/// le signale, R-131) : une hiérarchie réellement profonde ne coûte pas
/// plus. `OutlineLevel` est écrit tel quel.
fn outline_numbers(p: &Project) -> Vec<String> {
    let cap = usize::from(max_len(Parent::Task, "OutlineNumber").unwrap_or(512)).saturating_add(1);
    let mut levels: Vec<u16> = Vec::new();
    let mut counters: Vec<u32> = Vec::new();
    p.tasks
        .iter()
        .map(|t| {
            if t.is_null {
                return String::new();
            }
            let level = t.outline_level.get();
            while levels.last().is_some_and(|&l| l >= level) {
                levels.pop();
            }
            levels.push(level);
            let depth = levels.len();
            counters.truncate(depth);
            while counters.len() < depth {
                counters.push(0);
            }
            if let Some(last) = counters.last_mut() {
                *last = last.saturating_add(1);
            }
            let mut number = String::new();
            for (i, c) in counters.iter().enumerate() {
                if number.len() >= cap {
                    break;
                }
                if i > 0 {
                    number.push('.');
                }
                number.push_str(&c.to_string());
            }
            number
        })
        .collect()
}

/// R-127, R-214 : durée et travail restants écrits. Une feuille reçoit la
/// dérivation (ADR-0014 D4 : vérité = réels) : reste = D − durée réelle
/// effective (R-200, 0 pour une tâche achevée), travail restant = travail
/// − travail réel (R-211), jamais négatifs ; sans avancement, la durée et
/// le travail, comme MS Project les écrit. Une récapitulative garde ses
/// valeurs : sa durée est calculée (R-071), pas une entrée du modèle.
pub(crate) fn remaining(t: &Task, summary: bool) -> (Minutes, Work) {
    let p = &t.progress;
    if summary {
        return (p.remaining_duration, p.remaining_work);
    }
    let rest = t
        .work
        .minutes()
        .get()
        .saturating_sub(p.actual_work.minutes().get())
        .max(0);
    let work = Work::new(Minutes::new(rest)).unwrap_or(Work::ZERO);
    (planrs_core::progress::remaining_duration(t), work)
}

/// Durée écrite : calculée pour une feuille automatique affectée (R-199),
/// sinon celle du modèle.
fn duration_of(row: &Row<'_>) -> Duration {
    row.values
        .and_then(|v| v.leaf_duration)
        .unwrap_or(row.task.duration)
}

/// Restes écrits (R-127), sur la durée écrite (R-199) : l'image relue reste
/// celle de `normalize` appliquée à `schedule_image`.
fn remaining_of(row: &Row<'_>) -> (Minutes, Work) {
    match row.values.and_then(|v| v.leaf_duration) {
        Some(d) => {
            let mut t = row.task.clone();
            t.duration = d;
            remaining(&t, row.summary)
        }
        None => remaining(row.task, row.summary),
    }
}

/// `DurationFormat` d'une durée.
fn duration_format(d: &Duration) -> String {
    codes::duration_format_code(d.unit(), d.basis(), d.is_estimated()).to_string()
}

/// Valeurs saisies d'une tâche manuelle.
type ManualInputs = (
    Option<planrs_core::Moment>,
    Option<planrs_core::Moment>,
    Option<Duration>,
);

/// Écrit une tâche.
fn write(em: &mut Emitter, row: &Row<'_>, profile: OrderProfile) {
    let t = row.task;
    em.subject = Some(Subject::Task(t.uid()));
    let manual = match t.mode {
        SchedulingMode::Manual {
            start,
            finish,
            duration,
        } => Some((start, finish, duration)),
        SchedulingMode::Auto => None,
    };
    let numbers: Vec<u8> = t.baselines.keys().map(|n| n.get()).collect();
    let planned = plan(em, "Task", Parent::Task, &t.extras, &numbers, profile);
    let mut top = unscoped(&planned);
    leveling::drop_legacy(&mut top, t.leveling_delay);
    entity(em, "Task", Parent::Task, profile, &top, |em, name| {
        field(em, name, row, manual, &planned, profile);
    });
    em.subject = None;
}

/// Écrit l'élément `name` d'une tâche (rien s'il n'a pas de valeur).
fn field(
    em: &mut Emitter,
    name: &'static str,
    row: &Row<'_>,
    manual: Option<ManualInputs>,
    planned: &[Planned<'_>],
    profile: OrderProfile,
) {
    let t = row.task;
    let computed = row.values;
    let date = |d: Option<planrs_core::Moment>| d.map(format_datetime);
    match name {
        "UID" => em.leaf_int(name, i64::from(t.uid().get())),
        "GUID" => em.opt_leaf(name, t.guid.map(|g| g.to_string())),
        "ID" => em.leaf_int(name, i64::from(row.id)),
        "Name" => em.text(name, &t.name, max_len(Parent::Task, name)),
        "Active" => em.leaf(name, format_bool(t.active)),
        "Manual" => em.leaf(name, format_bool(manual.is_some())),
        "Type" => em.leaf_int(name, codes::task_type_code(t.task_type)),
        "IsNull" => em.leaf(name, format_bool(t.is_null)),
        "CreateDate" => em.opt_date(name, t.created),
        "WBS" => {
            if let Some(w) = &t.wbs {
                em.text(name, w, max_len(Parent::Task, name));
            }
        }
        "OutlineNumber" => em.text(name, &row.outline_number, max_len(Parent::Task, name)),
        "OutlineLevel" => em.leaf_int(name, i64::from(row.level)),
        "Priority" => em.leaf_int(name, i64::from(t.priority.get())),
        // Durée : entrée d'une feuille ou d'une tâche manuelle ; valeur
        // calculée d'une récapitulative automatique (R-071, R-122) et d'une
        // feuille automatique affectée (R-199).
        "Duration" => {
            if !row.summary || manual.is_some() {
                em.leaf_iso(name, duration_of(row).minutes().get());
            } else {
                em.opt_leaf(name, computed.and_then(|v| v.duration).map(format_iso_duration));
            }
        }
        // R-122 : valeurs calculées.
        "Start" => em.opt_date(name, computed.and_then(|v| v.start)),
        "Finish" => em.opt_date(name, computed.and_then(|v| v.finish)),
        "EarlyStart" => em.opt_date(name, computed.and_then(|v| v.early_start)),
        "EarlyFinish" => em.opt_date(name, computed.and_then(|v| v.early_finish)),
        "LateStart" => em.opt_date(name, computed.and_then(|v| v.late_start)),
        "LateFinish" => em.opt_date(name, computed.and_then(|v| v.late_finish)),
        "TotalSlack" => em.opt_int(name, computed.and_then(|v| v.total_slack)),
        "FreeSlack" => em.opt_int(name, computed.and_then(|v| v.free_slack)),
        "StartSlack" => em.opt_int(name, computed.and_then(|v| v.start_slack)),
        "FinishSlack" => em.opt_int(name, computed.and_then(|v| v.finish_slack)),
        "Critical" => em.opt_leaf(
            name,
            computed
                .and_then(|v| v.critical)
                .map(|b| format_bool(b).to_owned()),
        ),
        // R-129 : toujours écrits pour une tâche manuelle, vides si absents.
        "ManualStart" | "ManualFinish" | "ManualDuration" => {
            if let Some((start, finish, duration)) = manual {
                let value = match name {
                    "ManualStart" => date(start),
                    "ManualFinish" => date(finish),
                    _ => duration.map(|d| format_iso_duration(d.minutes().get())),
                };
                em.leaf(name, &value.unwrap_or_default());
            }
        }
        "DurationFormat" => em.leaf(name, &duration_format(&t.duration)),
        // R-199 : travail calculé d'une récapitulative et de l'UID 0 avec
        // `Computed::Schedule` (R-184), sinon celui du modèle.
        "Work" => em.leaf_iso(
            name,
            computed.and_then(|v| v.work).unwrap_or(t.work).minutes().get(),
        ),
        "EffortDriven" => em.leaf(name, format_bool(t.effort_driven)),
        "Recurring" => em.leaf(name, format_bool(t.recurring)),
        "Estimated" => em.leaf(name, format_bool(t.duration.is_estimated())),
        "Milestone" => em.leaf(name, format_bool(t.milestone)),
        "Summary" => em.leaf(name, format_bool(row.summary)),
        "PercentComplete" => percent(em, name, t.progress.percent_complete),
        "PercentWorkComplete" => percent(em, name, t.progress.percent_work_complete),
        "PhysicalPercentComplete" => percent(em, name, t.progress.physical_percent_complete),
        // R-214, R-215 : `Stop` calculé ; `Resume` calculé, ou la reprise
        // saisie d'une feuille en cours sans valeurs calculées.
        "Stop" => em.opt_date(name, computed.and_then(|v| v.stop)),
        "Resume" => em.opt_date(
            name,
            computed
                .and_then(|v| v.resume)
                .or_else(|| super::computed::model_resume(t, row.summary)),
        ),
        "ActualStart" => em.opt_date(name, t.progress.actual_start),
        "ActualFinish" => em.opt_date(name, t.progress.actual_finish),
        "ActualDuration" => em.leaf_iso(name, t.progress.actual_duration.get()),
        "ActualWork" => em.leaf_iso(name, t.progress.actual_work.minutes().get()),
        "RemainingDuration" => em.leaf_iso(name, remaining_of(row).0.get()),
        "RemainingWork" => em.leaf_iso(name, remaining_of(row).1.minutes().get()),
        "ConstraintType" => em.leaf_int(name, codes::constraint_code(t.constraint.kind())),
        "CalendarUID" => em.leaf(
            name,
            &t.calendar
                .map_or_else(|| "-1".to_owned(), |c| c.get().to_string()),
        ),
        // R-132 : date écrite pour les types 2 à 7 seulement.
        "ConstraintDate" => em.opt_date(name, constraint_date(t.constraint)),
        "Deadline" => em.opt_date(name, t.deadline),
        "IgnoreResourceCalendar" => em.leaf(name, format_bool(t.ignore_resource_calendar)),
        "LevelingDelay" | "LevelingDelayFormat" => leveling::write(em, name, t.leveling_delay),
        "Notes" => em.text(name, &t.notes, max_len(Parent::Task, name)),
        "PredecessorLink" => {
            for link in &t.predecessors {
                predecessor(em, link, profile);
            }
        }
        "Baseline" => baseline::task(em, &t.baselines, planned, profile),
        _ => {}
    }
}

/// Date d'une contrainte datée.
const fn constraint_date(c: Constraint) -> Option<planrs_core::Moment> {
    c.date()
}

/// Un lien (R-120) : `LinkLag` en dixièmes de minute (minutes × 10) ou,
/// en pourcentage, pourcentage × 10 (R-073 : 1/10 000 / 10) ; `LagFormat`
/// selon l'unité et la base.
fn predecessor(em: &mut Emitter, link: &Link, profile: OrderProfile) {
    let (lag, format) = lag_fields(em, &link.lag);
    entity(
        em,
        "PredecessorLink",
        Parent::PredecessorLink,
        profile,
        NO_EXTRAS,
        |em, name| match name {
            "PredecessorUID" => em.leaf(name, &link.predecessor.get().to_string()),
            "Type" => em.leaf(name, &codes::link_type_code(link.kind).to_string()),
            "CrossProject" => em.leaf(name, "0"),
            "LinkLag" => em.leaf(name, &lag.to_string()),
            "LagFormat" => em.leaf(name, &format.to_string()),
            _ => {}
        },
    );
}

/// `LinkLag` et `LagFormat` d'un décalage ; pourcentage au pas de 0,1 %
/// (arrondi R-001 signalé).
fn lag_fields(em: &mut Emitter, lag: &Lag) -> (i64, i64) {
    match lag.value {
        LagValue::Time { minutes, unit } => (
            minutes.get().saturating_mul(10),
            codes::duration_format_code(unit, lag.basis, false),
        ),
        LagValue::Percent { percent } => {
            let v = percent.get();
            if v.checked_rem(10).is_some_and(|r| r != 0) {
                em.report(WriteCode::Rounded, "LinkLag", Some(v.to_string()));
            }
            (
                planrs_core::round_div(i128::from(v), 10).unwrap_or(0),
                codes::percent_lag_format_code(lag.basis),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use planrs_core::{
        BuiltinCalendarNames, OutlineLevel, Project, ProjectInit, ScheduleAnchor, Task, TaskUid,
    };

    use super::{outline_numbers, summary_flags};

    #[test]
    fn r120_outline_numbers_and_summary_flags() {
        let mut p = Project::new(ProjectInit {
            name: "P".to_owned(),
            anchor: ScheduleAnchor::FromStart("2026-10-19T08:00".parse().unwrap()),
            current_date: "2026-10-17T08:00".parse().unwrap(),
            calendar_names: BuiltinCalendarNames::default(),
        });
        let o = p.options;
        for (uid, level, null) in [
            (1, 1, false),
            (2, 2, false),
            (3, 2, true),
            (4, 3, false),
            (5, 1, false),
        ] {
            let mut t = Task::new(TaskUid::new(uid), String::new(), &o);
            t.outline_level = OutlineLevel::new(level).unwrap();
            t.is_null = null;
            p.tasks.push(t).unwrap();
        }
        assert_eq!(outline_numbers(&p), ["1", "1.1", "", "1.1.1", "2"]);
        assert_eq!(summary_flags(&p), [true, true, false, false, false]);
    }

    /// Projet de feuilles aux niveaux donnés.
    fn with_levels(levels: impl IntoIterator<Item = u16>) -> Project {
        let mut p = Project::new(ProjectInit {
            name: "P".to_owned(),
            anchor: ScheduleAnchor::FromStart("2026-10-19T08:00".parse().unwrap()),
            current_date: "2026-10-17T08:00".parse().unwrap(),
            calendar_names: BuiltinCalendarNames::default(),
        });
        let o = p.options;
        for (uid, level) in (1..).zip(levels) {
            let mut t = Task::new(TaskUid::new(uid), String::new(), &o);
            t.outline_level = OutlineLevel::new(level).unwrap();
            p.tasks.push(t).unwrap();
        }
        p
    }

    /// R-120 : numérotation selon la hiérarchie du moteur (un niveau qui
    /// saute vaut le précédent + 1, même niveau = sœurs), longueur bornée
    /// (règle 10).
    #[test]
    fn r120_outline_numbers_clamp_level_jumps() {
        let p = with_levels([3, 9, 2, 65_535, 65_535, 1]);
        assert_eq!(outline_numbers(&p), ["1", "1.1", "2", "2.1", "2.2", "3"]);
        let numbers = outline_numbers(&with_levels([65_535; 2_000]));
        assert!(numbers.iter().all(|n| n.len() <= 4), "{:?}", numbers.last());
        assert_eq!(numbers.last().map(String::as_str), Some("2000"));
        // Hiérarchie réelle de 400 niveaux : numéro construit au plus
        // jusqu'au maxLength + 1 environ, tronqué ensuite par l'émetteur.
        let numbers = outline_numbers(&with_levels(1..=400));
        assert_eq!(numbers.get(2).map(String::as_str), Some("1.1.1"));
        assert!(numbers.iter().all(|n| n.len() <= 514));
        assert!(numbers.last().is_some_and(|n| n.len() > 512));
    }
}
