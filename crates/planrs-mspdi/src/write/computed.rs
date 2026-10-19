//! Valeurs calculées écrites (R-122, ADR-0010 D12) : depuis le `Schedule`
//! de planrs, depuis le planning relu (`RecordedSchedule`, projet non
//! modifié), ou omises (MS Project les calcule).

use std::collections::{BTreeMap, BTreeSet};

use planrs_core::calendar::{CalendarSet, CompiledCalendar, TimeScale, standard};
use planrs_core::schedule::measure_calendar_in;
use planrs_core::{
    Arena, AssignmentUid, Exclusion, Minutes, Moment, Project, ResourceKind, ResourceUid, Schedule,
    SchedulingMode, Task, TaskOutcome, TaskSchedule, TaskUid, TimeBasis, Work,
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
/// donnerait des dates fausses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScheduleGap {
    /// Avancement saisi (% achevé, date ou durée réelle) : J6.
    Progress,
    /// Ressource de travail dotée d'un calendrier affectée à une feuille
    /// automatique active qui ne l'ignore pas : J5.
    ResourceCalendars,
    /// Retard de nivellement non nul (modèle, R-198, ou `LevelingDelay`
    /// conservé en `Extras` par une version antérieure) : J5.
    LevelingDelay,
}

impl ScheduleGap {
    /// Code stable (`progress`…).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Progress => "progress",
            Self::ResourceCalendars => "resource_calendars",
            Self::LevelingDelay => "leveling_delay",
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
    let leaves = automatic_active_leaves(project);
    let calendar_resources: BTreeSet<_> = project
        .resources
        .iter()
        .filter(|r| r.kind == ResourceKind::Work && r.calendar.is_some())
        .map(planrs_core::Resource::uid)
        .collect();
    if project.assignments.iter().any(|a| {
        a.resource.is_some_and(|r| calendar_resources.contains(&r))
            && leaves.contains(&a.task)
            && project
                .tasks
                .get(a.task)
                .is_some_and(|t| !t.ignore_resource_calendar)
    }) {
        gaps.insert(ScheduleGap::ResourceCalendars);
    }
    let delayed = |extras: &planrs_core::Extras| {
        extras.0.iter().any(|e| {
            e.origin == ORIGIN
                && e.key == "LevelingDelay"
                && leveling_delay(&e.payload).is_some_and(|v| v != 0)
        })
    };
    let modeled = |d: Option<planrs_core::LevelingDelay>| d.is_some_and(|d| d.minutes.get() != 0);
    if tasks().any(|t| modeled(t.leveling_delay) || delayed(&t.extras))
        || project
            .assignments
            .iter()
            .any(|a| modeled(a.leveling_delay) || delayed(&a.extras))
    {
        gaps.insert(ScheduleGap::LevelingDelay);
    }
    gaps.into_iter().collect()
}

/// Valeur entière d'une charge `<LevelingDelay>n</LevelingDelay>`.
pub(crate) fn leveling_delay(payload: &str) -> Option<i64> {
    let start = payload.find('>')?.checked_add(1)?;
    let end = payload.rfind("</")?;
    parse_int(payload.get(start..end)?)
}

/// Feuilles automatiques actives (R-052 : une feuille n'a pas de tâche non
/// vide de niveau supérieur juste après elle).
fn automatic_active_leaves(project: &Project) -> BTreeSet<TaskUid> {
    let mut out = BTreeSet::new();
    let mut next: Option<u16> = None;
    for t in project.tasks.iter().rev() {
        if t.is_null {
            continue;
        }
        let level = t.outline_level.get();
        let leaf = next.is_none_or(|n| n <= level);
        next = Some(level);
        if leaf && t.active && matches!(t.mode, SchedulingMode::Auto) {
            out.insert(t.uid());
        }
    }
    out
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
}

/// Valeurs calculées d'une affectation, prêtes à écrire (R-198).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AssignmentValues {
    pub(crate) start: Option<Moment>,
    pub(crate) finish: Option<Moment>,
}

/// Valeurs calculées d'une ressource, prêtes à écrire (R-198).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ResourceValues {
    pub(crate) work: Option<Work>,
    pub(crate) overallocated: Option<bool>,
}

/// Valeurs calculées de tout le projet.
#[derive(Debug, Default)]
pub(crate) struct Values {
    pub(crate) tasks: BTreeMap<TaskUid, TaskValues>,
    /// Affectations : relues seulement (le moteur les donnera, R-199).
    pub(crate) assignments: BTreeMap<AssignmentUid, AssignmentValues>,
    /// Ressources : relues seulement (le moteur les donnera, R-199).
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
    }
}

/// Valeurs depuis le `Schedule` (R-122).
pub(crate) fn from_schedule(project: &Project, schedule: &Schedule) -> Values {
    let scales = Scales::new(project);
    let mut tasks = BTreeMap::new();
    for t in &project.tasks {
        let values = match schedule.get(t.uid()) {
            Some(TaskOutcome::Scheduled(s)) => {
                let summary = s.role == planrs_core::TaskRole::Summary;
                from_task_schedule(s, scales.of(t, summary), summary)
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
    Values {
        tasks,
        project_start: Some(schedule.project_start),
        project_finish: Some(schedule.project_finish),
        ..Values::default()
    }
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
