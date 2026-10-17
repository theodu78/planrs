//! Correspondance typée de l'élément `Project` (§ 7.6) : informations,
//! ancre, options, calendrier du projet ; puis calendriers, tâches,
//! ressources et affectations.
//!
//! Valeurs par défaut : celles du schéma MSPDI pour un élément absent
//! (R-076), sinon celles du modèle (R-032).

use std::collections::{BTreeMap, BTreeSet};

use planrs_core::{
    Arena, BuiltinCalendarNames, CalendarUid, ConversionFactors, ExtraEntry, Extras, Guid, Project,
    ProjectInfo, ProjectInit, ProjectOptions, ScheduleAnchor, Subject, TaskType, TaskUid,
};

use crate::ReadOptions;
use crate::assignment;
use crate::calendar;
use crate::codes::{self, DurationCode};
use crate::diag::{ImportCode, Sink};
use crate::extras::{self, ORIGIN};
use crate::fields::Fields;
use crate::recorded::RecordedSchedule;
use crate::resource;
use crate::schema::Parent;
use crate::task;
use crate::xml::RawDocument;

/// Résultat de la correspondance.
pub(crate) struct Built {
    pub(crate) project: Project,
    pub(crate) recorded: RecordedSchedule,
    pub(crate) save_version: Option<i64>,
    /// Calculés écartés après la lecture XML (répartitions de type 1 des
    /// affectations, R-087), à ajouter à ceux de la lecture.
    pub(crate) dropped_computed: BTreeMap<String, u64>,
}

/// UID du calendrier du projet quand `CalendarUID` manque et qu'aucun
/// calendrier n'est lu (hypothèse : celui du Standard d'un projet neuf).
const FALLBACK_CALENDAR: CalendarUid = CalendarUid::new(1);

/// Construit le projet et le planning enregistré.
pub(crate) fn build(raw: &RawDocument<'_>, options: &ReadOptions, sink: &mut Sink<'_>) -> Built {
    let f = Fields::new(raw.fields.iter(), "", Some(Subject::Project), raw.root_at);
    let save_version = f.int("SaveVersion", sink).map(|(v, _)| v);

    let recorded_start = f.datetime("StartDate", sink);
    if !f.has("StartDate") {
        f.missing("StartDate", sink);
    }
    let start = recorded_start.unwrap_or(options.current_date);
    let recorded_finish = f.datetime("FinishDate", sink);
    let current_date = f.datetime("CurrentDate", sink).unwrap_or(options.current_date);
    let anchor = if f.bool("ScheduleFromStart", sink).unwrap_or(true) {
        ScheduleAnchor::FromStart(start)
    } else {
        f.report(
            ImportCode::Unsupported {
                what: "schedule_from_finish".to_owned(),
            },
            "ScheduleFromStart",
            f.at(),
            None,
            sink,
        );
        ScheduleAnchor::FromFinish(recorded_finish.unwrap_or(start))
    };

    let mut project = Project::new(ProjectInit {
        name: String::new(),
        anchor,
        current_date,
        calendar_names: BuiltinCalendarNames::default(),
    });
    // Les calendriers sont ceux du fichier, jamais ceux d'un projet neuf.
    project.calendars = Arena::new();
    project.info = info(&f, sink);
    project.status_date = f.datetime("StatusDate", sink);
    project.options = project_options(&f, sink);
    project.calendar = project_calendar(&f, sink);
    project.calendars = calendar::import(&raw.calendars, project.calendar, sink);
    project.extras = project_extras(raw);
    let mut recorded = RecordedSchedule {
        project_start: recorded_start,
        project_finish: recorded_finish,
        tasks: BTreeMap::new(),
    };

    task::import(&raw.tasks, &mut project, &mut recorded, sink);
    project.resources = resource::import(&raw.resources, &project.options.factors, sink);
    let tasks: BTreeSet<TaskUid> = recorded.tasks.keys().copied().collect();
    let (assignments, dropped_timephased) = assignment::import(&raw.assignments, &tasks, sink);
    for (uid, n) in assignment::count_by_task(&assignments) {
        if let Some(rec) = recorded.tasks.get_mut(&uid) {
            rec.assignments = n;
        }
    }
    project.assignments = assignments;
    let mut dropped_computed = BTreeMap::new();
    if dropped_timephased > 0 {
        dropped_computed.insert("TimephasedData".to_owned(), dropped_timephased);
    }

    Built {
        project,
        recorded,
        save_version,
        dropped_computed,
    }
}

/// `Extras` du projet (R-087) : déclarations `xmlns:préfixe` de la racine,
/// puis enfants capturés de `Project`, dans l'ordre canonique.
fn project_extras(raw: &RawDocument<'_>) -> Extras {
    let mut out: Vec<ExtraEntry> = raw
        .prefixes
        .iter()
        .map(|(prefix, uri)| ExtraEntry {
            origin: ORIGIN.to_owned(),
            key: format!("@xmlns:{prefix}"),
            payload: uri.clone(),
        })
        .collect();
    out.extend(extras::collect(&raw.fields, Parent::Project, None));
    let mut extras = Extras(out);
    extras::canonical_order(&mut extras, Parent::Project);
    extras
}

/// Informations descriptives.
fn info(f: &Fields<'_>, sink: &mut Sink<'_>) -> ProjectInfo {
    let text = |name: &str, sink: &mut Sink<'_>| f.raw(name, sink).unwrap_or_default().to_owned();
    ProjectInfo {
        guid: guid(f, "GUID", sink),
        name: text("Name", sink),
        title: text("Title", sink),
        subject: text("Subject", sink),
        author: text("Author", sink),
        manager: text("Manager", sink),
        company: text("Company", sink),
        category: text("Category", sink),
        creation_date: f.datetime("CreationDate", sink),
    }
}

/// GUID `8-4-4-4-12` ; illisible → diagnostic, aucun.
pub(crate) fn guid(f: &Fields<'_>, name: &str, sink: &mut Sink<'_>) -> Option<Guid> {
    let (t, at) = f.scalar(name, sink)?;
    let g = t.parse().ok();
    if g.is_none() {
        f.invalid(name, at, t, sink);
    }
    g
}

/// Entier borné ; absent → `default` ; hors bornes ou illisible →
/// diagnostic et `default`.
fn bounded_u32(
    f: &Fields<'_>,
    name: &str,
    range: std::ops::RangeInclusive<u32>,
    default: u32,
    sink: &mut Sink<'_>,
) -> u32 {
    match f.int(name, sink) {
        None => default,
        Some((v, at)) => match u32::try_from(v) {
            Ok(v) if range.contains(&v) => v,
            _ => {
                f.invalid(name, at, &v.to_string(), sink);
                default
            }
        },
    }
}

/// Code numérique traduit par `table` ; absent → `None` ; inconnu →
/// diagnostic `mspdi.unknown_code`, `None`.
pub(crate) fn code<T>(
    f: &Fields<'_>,
    name: &str,
    table: impl Fn(i64) -> Option<T>,
    sink: &mut Sink<'_>,
) -> Option<T> {
    let (v, at) = f.int(name, sink)?;
    let out = table(v);
    if out.is_none() {
        f.report(ImportCode::UnknownCode { code: v }, name, at, None, sink);
    }
    out
}

/// Options du projet (R-076 pour les absents).
fn project_options(f: &Fields<'_>, sink: &mut Sink<'_>) -> ProjectOptions {
    let defaults = ProjectOptions::default();
    let base = ConversionFactors::default();
    let per_day = bounded_u32(f, "MinutesPerDay", 1..=1_440, base.minutes_per_day(), sink);
    let per_week = bounded_u32(f, "MinutesPerWeek", 1..=10_080, base.minutes_per_week(), sink);
    let per_month = bounded_u32(f, "DaysPerMonth", 1..=31, base.days_per_month(), sink);
    let factors = ConversionFactors::new(per_day, per_week, per_month).unwrap_or(base);

    let duration_unit = match code(f, "DurationFormat", codes::duration_format, sink) {
        Some(DurationCode::Time { unit, .. }) => unit,
        Some(DurationCode::Null { .. }) | None => defaults.default_duration_unit,
        Some(DurationCode::Percent { .. }) => {
            if let Some((t, at)) = f.scalar("DurationFormat", sink) {
                f.invalid("DurationFormat", at, t, sink);
            }
            defaults.default_duration_unit
        }
    };
    ProjectOptions {
        factors,
        default_start_time: f
            .time("DefaultStartTime", sink)
            .unwrap_or(defaults.default_start_time),
        default_finish_time: f
            .time("DefaultFinishTime", sink)
            .unwrap_or(defaults.default_finish_time),
        default_duration_unit: duration_unit,
        default_work_unit: code(f, "WorkFormat", codes::work_format, sink)
            .unwrap_or(defaults.default_work_unit),
        // R-076 : valeurs `default=` du schéma pour un élément absent.
        default_task_type: code(f, "DefaultTaskType", codes::task_type, sink)
            .unwrap_or(TaskType::FixedDuration),
        new_tasks_effort_driven: f.bool("NewTasksEffortDriven", sink).unwrap_or(true),
        new_tasks_estimated: f.bool("NewTasksEstimated", sink).unwrap_or(true),
        week_start_day: code(f, "WeekStartDay", codes::week_start_day, sink)
            .unwrap_or(defaults.week_start_day),
        honor_constraints: f.bool("HonorConstraints", sink).unwrap_or(true),
        // R-086 : seuil de criticité en jours, entier >= 0 ; un seul chemin
        // critique par défaut.
        critical_slack_limit_days: bounded_u32(f, "CriticalSlackLimit", 0..=u32::MAX, 0, sink),
        multiple_critical_paths: f.bool("MultipleCriticalPaths", sink).unwrap_or(false),
    }
}

/// Calendrier du projet (`CalendarUID`).
fn project_calendar(f: &Fields<'_>, sink: &mut Sink<'_>) -> CalendarUid {
    match f.int("CalendarUID", sink) {
        None => {
            if !f.has("CalendarUID") {
                f.missing("CalendarUID", sink);
            }
            FALLBACK_CALENDAR
        }
        Some((v, at)) => u32::try_from(v).map_or_else(
            |_| {
                f.invalid("CalendarUID", at, &v.to_string(), sink);
                FALLBACK_CALENDAR
            },
            CalendarUid::new,
        ),
    }
}
