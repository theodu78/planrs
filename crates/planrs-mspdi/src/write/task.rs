//! Tâches (R-120, R-127, R-129, R-132) : champs modélisés, liens, mode
//! manuel ; ID, numéro hiérarchique et drapeau `Summary` déduits.

use planrs_core::{Constraint, Duration, Lag, LagValue, Link, Project, SchedulingMode, Subject, Task};

use super::emit::Emitter;
use super::{OrderProfile, WriteCode, entity, max_len, percent};
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
}

/// Écrit la récapitulative du projet puis les tâches, dans l'ordre des ID.
pub(crate) fn write_all(em: &mut Emitter, p: &Project, profile: OrderProfile) {
    if let Some(summary) = &p.summary_task {
        let row = Row {
            task: summary,
            id: 0,
            summary: true,
            level: 0,
            outline_number: "0".to_owned(),
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
/// vide reçoit celui de sa position sans faire avancer les compteurs.
fn outline_numbers(p: &Project) -> Vec<String> {
    let mut counters: Vec<u32> = Vec::new();
    p.tasks
        .iter()
        .map(|t| {
            let level = usize::from(t.outline_level.get());
            if t.is_null {
                return String::new();
            }
            counters.truncate(level);
            while counters.len() < level {
                counters.push(0);
            }
            if let Some(last) = counters.last_mut() {
                *last = last.saturating_add(1);
            }
            counters.iter().map(u32::to_string).collect::<Vec<_>>().join(".")
        })
        .collect()
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
    entity(em, "Task", Parent::Task, profile, |em, name| {
        field(em, name, row, manual, profile);
    });
    em.subject = None;
}

/// Écrit l'élément `name` d'une tâche (rien s'il n'a pas de valeur).
fn field(
    em: &mut Emitter,
    name: &'static str,
    row: &Row<'_>,
    manual: Option<ManualInputs>,
    profile: OrderProfile,
) {
    let t = row.task;
    let date = |d: Option<planrs_core::Moment>| d.map(format_datetime);
    match name {
        "UID" => em.leaf(name, &t.uid().get().to_string()),
        "GUID" => em.opt_leaf(name, t.guid.map(|g| g.to_string())),
        "ID" => em.leaf(name, &row.id.to_string()),
        "Name" => em.text(name, &t.name, max_len(Parent::Task, name)),
        "Active" => em.leaf(name, format_bool(t.active)),
        "Manual" => em.leaf(name, format_bool(manual.is_some())),
        "Type" => em.leaf(name, &codes::task_type_code(t.task_type).to_string()),
        "IsNull" => em.leaf(name, format_bool(t.is_null)),
        "CreateDate" => em.opt_leaf(name, date(t.created)),
        "WBS" => {
            if let Some(w) = &t.wbs {
                em.text(name, w, max_len(Parent::Task, name));
            }
        }
        "OutlineNumber" => em.text(name, &row.outline_number, max_len(Parent::Task, name)),
        "OutlineLevel" => em.leaf(name, &row.level.to_string()),
        "Priority" => em.leaf(name, &t.priority.get().to_string()),
        // Durée : entrée d'une feuille ou d'une tâche manuelle ; valeur
        // calculée d'une récapitulative automatique (R-071, R-122).
        "Duration" => {
            if !row.summary || manual.is_some() {
                em.leaf(name, &format_iso_duration(t.duration.minutes().get()));
            }
        }
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
        "Work" => em.leaf(name, &format_iso_duration(t.work.minutes().get())),
        "EffortDriven" => em.leaf(name, format_bool(t.effort_driven)),
        "Recurring" => em.leaf(name, format_bool(t.recurring)),
        "Estimated" => em.leaf(name, format_bool(t.duration.is_estimated())),
        "Milestone" => em.leaf(name, format_bool(t.milestone)),
        "Summary" => em.leaf(name, format_bool(row.summary)),
        "PercentComplete" => percent(em, name, t.progress.percent_complete),
        "PercentWorkComplete" => percent(em, name, t.progress.percent_work_complete),
        "PhysicalPercentComplete" => percent(em, name, t.progress.physical_percent_complete),
        "ActualStart" => em.opt_leaf(name, date(t.progress.actual_start)),
        "ActualFinish" => em.opt_leaf(name, date(t.progress.actual_finish)),
        "ActualDuration" => em.leaf(name, &format_iso_duration(t.progress.actual_duration.get())),
        "ActualWork" => em.leaf(name, &format_iso_duration(t.progress.actual_work.minutes().get())),
        "RemainingDuration" => em.leaf(name, &format_iso_duration(t.progress.remaining_duration.get())),
        "RemainingWork" => em.leaf(
            name,
            &format_iso_duration(t.progress.remaining_work.minutes().get()),
        ),
        "ConstraintType" => em.leaf(name, &codes::constraint_code(t.constraint.kind()).to_string()),
        "CalendarUID" => em.leaf(
            name,
            &t.calendar
                .map_or_else(|| "-1".to_owned(), |c| c.get().to_string()),
        ),
        // R-132 : date écrite pour les types 2 à 7 seulement.
        "ConstraintDate" => em.opt_leaf(name, date(constraint_date(t.constraint))),
        "Deadline" => em.opt_leaf(name, date(t.deadline)),
        "IgnoreResourceCalendar" => em.leaf(name, format_bool(t.ignore_resource_calendar)),
        "Notes" => em.text(name, &t.notes, max_len(Parent::Task, name)),
        "PredecessorLink" => {
            for link in &t.predecessors {
                predecessor(em, link, profile);
            }
        }
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
}
