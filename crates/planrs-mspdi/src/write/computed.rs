//! Valeurs calculées écrites (R-122, ADR-0010 D12) : depuis le `Schedule`
//! de planrs, depuis le planning relu (`RecordedSchedule`, projet non
//! modifié), ou omises (MS Project les calcule).

use std::collections::{BTreeMap, BTreeSet};

use planrs_core::calendar::{CalendarSet, CompiledCalendar, TimeScale, standard};
use planrs_core::schedule::measure_calendar_in;
use planrs_core::{
    Arena, AssignmentRole, AssignmentUid, Duration, Exclusion, Minutes, Moment, Project, ResourceUid,
    Schedule, SchedulingMode, Task, TaskOutcome, TaskRole, TaskSchedule, TaskType, TaskUid, TimeBasis, Work,
};

use super::WriteDiagnostic;
use crate::extras::ORIGIN;
use crate::recorded::{RecordedSchedule, RecordedTask, Tenths};
use crate::scalar::parse_int;

/// Source des valeurs calculées écrites.
#[derive(Clone, Copy, Debug)]
pub enum Computed<'a> {
    /// Valeurs du moteur (`schedule(&project)`).
    Schedule(&'a Schedule),
    /// Valeurs relues, réécrites telles quelles (projet non modifié depuis
    /// la lecture).
    Recorded(&'a RecordedSchedule),
    /// Aucune valeur calculée : MS Project les calcule à l'ouverture.
    Omit,
}

/// Ce que le planning de planrs ne calcule pas encore (R-122) : l'écrire
/// donnerait des dates fausses. R-122 amendée (J5, R-199) : calendriers de
/// ressource et retards de nivellement sont calculés (R-171, R-178), seuls
/// restent les manques d'avancement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScheduleGap {
    /// Avancement saisi (% achevé, date ou durée réelle) : J6.
    Progress,
}

impl ScheduleGap {
    /// Code stable (`progress`).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Progress => "progress",
        }
    }
}

/// R-122 : manques du planning de planrs pour ce projet, sans doublon,
/// dans l'ordre de [`ScheduleGap`].
#[must_use]
pub fn schedule_gaps(project: &Project) -> Vec<ScheduleGap> {
    let mut gaps = BTreeSet::new();
    let tasks = || project.summary_task.iter().chain(project.tasks.iter());
    if tasks().any(|t| {
        let p = &t.progress;
        p.percent_complete.get() > 0
            || p.actual_start.is_some()
            || p.actual_finish.is_some()
            || p.actual_duration.get() > 0
    }) {
        gaps.insert(ScheduleGap::Progress);
    }
    gaps.into_iter().collect()
}

/// Valeur entière d'une charge `<LevelingDelay>n</LevelingDelay>`.
pub(crate) fn leveling_delay(payload: &str) -> Option<i64> {
    let start = payload.find('>')?.checked_add(1)?;
    let end = payload.rfind("</")?;
    parse_int(payload.get(start..end)?)
}

/// R-122 : `Schedule` si le planning de planrs est complet pour ce projet ;
/// sinon les valeurs relues si elles sont fournies ; sinon rien, avec
/// `mspdi.write.schedule_incomplete`.
#[must_use]
pub fn choose_computed<'a>(
    project: &Project,
    schedule: &'a Schedule,
    recorded: Option<&'a RecordedSchedule>,
) -> (Computed<'a>, Vec<WriteDiagnostic>) {
    let gaps = schedule_gaps(project);
    if gaps.is_empty() {
        return (Computed::Schedule(schedule), Vec::new());
    }
    if let Some(r) = recorded {
        return (Computed::Recorded(r), Vec::new());
    }
    let d = WriteDiagnostic {
        code: super::WriteCode::ScheduleIncomplete { gaps },
        subject: None,
        element: String::new(),
        value: None,
    };
    (Computed::Omit, vec![d])
}

/// Valeurs calculées d'une tâche, prêtes à écrire.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TaskValues {
    pub(crate) start: Option<Moment>,
    pub(crate) finish: Option<Moment>,
    pub(crate) early_start: Option<Moment>,
    pub(crate) early_finish: Option<Moment>,
    pub(crate) late_start: Option<Moment>,
    pub(crate) late_finish: Option<Moment>,
    pub(crate) total_slack: Option<i64>,
    pub(crate) free_slack: Option<i64>,
    pub(crate) start_slack: Option<i64>,
    pub(crate) finish_slack: Option<i64>,
    pub(crate) critical: Option<bool>,
    /// Durée calculée d'une récapitulative automatique, en minutes.
    pub(crate) duration: Option<i64>,
    /// R-199 : travail calculé d'une récapitulative et de l'UID 0 (R-184),
    /// avec `Computed::Schedule` seulement.
    pub(crate) work: Option<Work>,
    /// R-199 : durée calculée d'une feuille automatique affectée (R-173),
    /// avec `Computed::Schedule` seulement (voir [`assigned_leaf_duration`]).
    pub(crate) leaf_duration: Option<Duration>,
}

/// Valeurs calculées d'une affectation, prêtes à écrire (R-198, R-199).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AssignmentValues {
    pub(crate) start: Option<Moment>,
    pub(crate) finish: Option<Moment>,
    /// R-199 : `TimephasedData` propres à jeter (placement changé).
    pub(crate) drop_timephased: bool,
}

/// Valeurs calculées d'une ressource, prêtes à écrire (R-198, R-199).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ResourceValues {
    pub(crate) work: Option<Work>,
    pub(crate) overallocated: Option<bool>,
}

/// Valeurs calculées de tout le projet.
#[derive(Debug, Default)]
pub(crate) struct Values {
    pub(crate) tasks: BTreeMap<TaskUid, TaskValues>,
    /// Affectations : relues, ou placées par le moteur (R-199).
    pub(crate) assignments: BTreeMap<AssignmentUid, AssignmentValues>,
    /// Ressources : relues, ou charges du moteur (R-199).
    pub(crate) resources: BTreeMap<ResourceUid, ResourceValues>,
    pub(crate) project_start: Option<Moment>,
    pub(crate) project_finish: Option<Moment>,
}

/// Dixièmes de minute d'une marge (minutes × 10) ; débordement : omise.
fn tenths(m: Minutes) -> Option<i64> {
    m.get().checked_mul(10)
}

/// Échelles de marge (R-048, R-066, R-067, R-113) : calendrier de mesure
/// du moteur (`planrs_core::schedule::measure_calendar_in` : calendrier
/// propre utilisable, sinon calendrier du projet — sa base s'il est dérivé —,
/// sinon le Standard intégré) ; temps civil pour une feuille à durée
/// écoulée.
struct Scales<'p> {
    project: &'p Project,
    set: CalendarSet,
    fallback: Option<CompiledCalendar>,
}

impl<'p> Scales<'p> {
    fn new(project: &'p Project) -> Self {
        let (set, _) = CalendarSet::compile(project);
        let usable = set
            .get(project.calendar)
            .is_ok_and(CompiledCalendar::has_working_time);
        let fallback = if usable {
            None
        } else {
            let std = standard(project.calendar, "Standard".to_owned());
            Arena::try_from(vec![std])
                .ok()
                .and_then(|a| CompiledCalendar::compile(&a, project.calendar).ok())
        };
        Self {
            project,
            set,
            fallback,
        }
    }

    /// Échelle de mesure de `task` ; `summary` : récapitulative (sa marge se
    /// mesure sur un calendrier, jamais en temps écoulé, comme le moteur).
    fn of(&self, task: &Task, summary: bool) -> Option<TimeScale<'_>> {
        if !summary && task.duration.basis() == TimeBasis::Elapsed {
            return Some(TimeScale::Elapsed);
        }
        let cal = match measure_calendar_in(self.project, &self.set, task) {
            Some(uid) => self.set.get(uid).ok(),
            None => self.fallback.as_ref(),
        };
        cal.map(TimeScale::Working)
    }
}

/// `StartSlack`, `FinishSlack` (dixièmes) sur l'échelle de marge, gardés
/// seulement si leur minimum redonne la marge totale du moteur (sinon
/// l'échelle supposée n'est pas la sienne : omis).
fn start_finish_slack(scale: Option<TimeScale<'_>>, s: &TaskSchedule) -> (Option<i64>, Option<i64>) {
    let Some(scale) = scale else {
        return (None, None);
    };
    let ss = scale.between(s.early.start, s.late.start).ok();
    let fs = scale.between(s.early.finish, s.late.finish).ok();
    match (ss, fs) {
        (Some(a), Some(b)) if a.min(b) == s.total_slack => (tenths(a), tenths(b)),
        _ => (None, None),
    }
}

fn from_task_schedule(s: &TaskSchedule, scale: Option<TimeScale<'_>>, summary: bool) -> TaskValues {
    let (start_slack, finish_slack) = start_finish_slack(scale, s);
    TaskValues {
        start: Some(s.start),
        finish: Some(s.finish),
        early_start: Some(s.early.start),
        early_finish: Some(s.early.finish),
        late_start: Some(s.late.start),
        late_finish: Some(s.late.finish),
        total_slack: tenths(s.total_slack),
        free_slack: tenths(s.free_slack),
        start_slack,
        finish_slack,
        critical: Some(s.critical),
        duration: (summary && !s.manual).then_some(s.duration.get()),
        work: summary.then_some(s.work),
        leaf_duration: None,
    }
}

/// Tâches dont une affectation de travail place les dates (rôle `Work`,
/// R-173).
fn placed_by_assignments(schedule: &Schedule) -> BTreeSet<TaskUid> {
    schedule
        .assignments
        .values()
        .filter(|a| a.role == AssignmentRole::Work)
        .map(|a| a.task)
        .collect()
}

/// R-199 : durée écrite d'une feuille automatique placée par ses
/// affectations de travail (`placed`), non dégradée : la durée calculée du
/// moteur (mesure de l'union des intervalles travaillés, R-173), avec
/// l'unité, la base et l'estimation de la tâche. Durée fixe exclue : la
/// durée y est l'entrée tenue (S41, ADR-0013 § 0). `None` : la durée du
/// modèle est écrite.
pub(crate) fn assigned_leaf_duration(
    task: &Task,
    s: &TaskSchedule,
    placed: &BTreeSet<TaskUid>,
) -> Option<Duration> {
    let eligible = s.role == TaskRole::Leaf
        && !s.manual
        && !s.degraded
        && matches!(task.mode, SchedulingMode::Auto)
        && task.task_type != TaskType::FixedDuration
        && placed.contains(&task.uid());
    if !eligible {
        return None;
    }
    let d = &task.duration;
    Duration::new(s.duration, d.unit(), d.basis(), d.is_estimated()).ok()
}

/// Valeurs depuis le `Schedule` (R-122).
pub(crate) fn from_schedule(project: &Project, schedule: &Schedule) -> Values {
    let scales = Scales::new(project);
    let placed = placed_by_assignments(schedule);
    let mut tasks = BTreeMap::new();
    for t in &project.tasks {
        let values = match schedule.get(t.uid()) {
            Some(TaskOutcome::Scheduled(s)) => {
                let summary = s.role == TaskRole::Summary;
                TaskValues {
                    leaf_duration: assigned_leaf_duration(t, s, &placed),
                    ..from_task_schedule(s, scales.of(t, summary), summary)
                }
            }
            Some(TaskOutcome::Excluded {
                reason: Exclusion::Inactive,
                display: Some(span),
            }) => TaskValues {
                start: Some(span.start),
                finish: Some(span.finish),
                ..TaskValues::default()
            },
            _ => TaskValues::default(),
        };
        tasks.insert(t.uid(), values);
    }
    if let (Some(summary), Some(s)) = (&project.summary_task, &schedule.project_summary) {
        tasks.insert(
            TaskUid::PROJECT_SUMMARY,
            from_task_schedule(s, scales.of(summary, true), true),
        );
    }
    // R-199 : `Start`/`Finish` de chaque affectation placée ; répartition
    // gardée seulement si son placement est celui du moteur.
    let assignments = project
        .assignments
        .iter()
        .map(|a| {
            let placed = schedule.assignments.get(&a.uid());
            let values = AssignmentValues {
                start: placed.map(|p| p.span.start),
                finish: placed.map(|p| p.span.finish),
                drop_timephased: super::timephased::stale(a, placed),
            };
            (a.uid(), values)
        })
        .collect();
    // R-199 : travail et surutilisation de chaque ressource chargée
    // (`PeakUnits` jamais écrit, O31).
    let resources = schedule
        .resources
        .iter()
        .map(|(uid, load)| {
            (
                *uid,
                ResourceValues {
                    work: Some(load.work),
                    overallocated: Some(load.overallocated),
                },
            )
        })
        .collect();
    Values {
        tasks,
        assignments,
        resources,
        project_start: Some(schedule.project_start),
        project_finish: Some(schedule.project_finish),
    }
}

/// R-199 : image du projet par une écriture avec `Computed::Schedule` (à
/// composer avec [`crate::normalize`] pour l'aller-retour) : les champs du
/// modèle que l'écrivain remplace par des valeurs du moteur. Travail d'une
/// récapitulative planifiée et de l'UID 0 ← R-184 ; durée d'une feuille
/// automatique affectée ← durée calculée (R-173, voir
/// [`assigned_leaf_duration`]) ; `TimephasedData` d'une affectation dont le
/// placement a changé retirées. Un `Schedule` d'un autre projet ne change
/// rien (`mspdi.write.schedule_mismatch`, aucune valeur écrite).
#[must_use]
pub fn schedule_image(project: &Project, schedule: &Schedule) -> Project {
    let mut p = project.clone();
    if !matches(project, schedule) {
        return p;
    }
    let placed = placed_by_assignments(schedule);
    let uids: Vec<TaskUid> = p.tasks.iter().map(Task::uid).collect();
    for uid in uids {
        let (Some(t), Some(TaskOutcome::Scheduled(s))) = (p.tasks.get_mut(uid), schedule.get(uid)) else {
            continue;
        };
        if s.role == TaskRole::Summary {
            t.work = s.work;
        } else if let Some(d) = assigned_leaf_duration(t, s, &placed) {
            t.duration = d;
        }
    }
    if let (Some(t), Some(s)) = (&mut p.summary_task, &schedule.project_summary) {
        t.work = s.work;
    }
    let auids: Vec<AssignmentUid> = p.assignments.iter().map(planrs_core::Assignment::uid).collect();
    for uid in auids {
        let Some(a) = p.assignments.get_mut(uid) else {
            continue;
        };
        if super::timephased::stale(a, schedule.assignments.get(&uid)) {
            a.extras.0.retain(|e| {
                !(e.origin == ORIGIN && super::timephased::is_scheduled_work(&e.key, &e.payload))
            });
        }
    }
    p
}

/// Valeurs relues, telles quelles (R-122).
pub(crate) fn from_recorded(recorded: &RecordedSchedule) -> Values {
    let tasks = recorded
        .tasks
        .iter()
        .map(|(uid, r)| (*uid, recorded_values(r)))
        .collect();
    let assignments = recorded
        .assignments
        .iter()
        .map(|(uid, r)| {
            (
                *uid,
                AssignmentValues {
                    start: r.start,
                    finish: r.finish,
                    drop_timephased: false,
                },
            )
        })
        .collect();
    let resources = recorded
        .resources
        .iter()
        .map(|(uid, r)| {
            (
                *uid,
                ResourceValues {
                    work: r.work,
                    overallocated: r.overallocated,
                },
            )
        })
        .collect();
    Values {
        tasks,
        assignments,
        resources,
        project_start: recorded.project_start,
        project_finish: recorded.project_finish,
    }
}

fn recorded_values(r: &RecordedTask) -> TaskValues {
    let t = |v: Option<Tenths>| v.map(|Tenths(x)| x);
    TaskValues {
        start: r.start,
        finish: r.finish,
        early_start: r.early_start,
        early_finish: r.early_finish,
        late_start: r.late_start,
        late_finish: r.late_finish,
        total_slack: t(r.total_slack),
        free_slack: t(r.free_slack),
        start_slack: t(r.start_slack),
        finish_slack: t(r.finish_slack),
        critical: r.critical,
        duration: r.duration.map(Minutes::get),
        ..TaskValues::default()
    }
}

/// Le `Schedule` couvre exactement les tâches du projet (UID 0 exclu).
pub(crate) fn matches(project: &Project, schedule: &Schedule) -> bool {
    schedule.tasks.len() == project.tasks.len()
        && project
            .tasks
            .iter()
            .all(|t| schedule.tasks.contains_key(&t.uid()))
}
