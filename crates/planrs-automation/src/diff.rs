//! Diff de planning entre deux états (R-163, lot L2).

use std::cmp::Ordering;
use std::collections::btree_map;
use std::iter::Peekable;

use planrs_core::{Minutes, Moment, Project, Schedule, TaskId, TaskOutcome, TaskSchedule, TaskUid};
use serde::Serialize;

/// Diff de planning d'un lot (R-163).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScheduleDiff {
    /// Fin du projet (avant, après).
    pub project_finish: (Moment, Moment),
    /// Début du projet (avant, après).
    pub project_start: (Moment, Moment),
    /// Tâches devenues critiques, par UID.
    pub critical_entered: Vec<TaskUid>,
    /// Tâches qui ne sont plus critiques, par UID.
    pub critical_left: Vec<TaskUid>,
    /// Nombre exact de tâches changées.
    pub changed_total: usize,
    /// Tâches changées, triées, au plus N.
    pub changed: Vec<TaskChange>,
    /// Vrai si `changed` a été coupé.
    pub truncated: bool,
}

/// Changement d'une tâche (R-163). Chaque paire est (avant, après).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TaskChange {
    /// UID.
    pub uid: TaskUid,
    /// ID avant (absent : tâche ajoutée).
    pub id_before: Option<TaskId>,
    /// ID après (absent : tâche supprimée).
    pub id_after: Option<TaskId>,
    /// Début planifié.
    pub start: (Option<Moment>, Option<Moment>),
    /// Fin planifiée.
    pub finish: (Option<Moment>, Option<Moment>),
    /// Marge totale.
    pub total_slack: (Option<Minutes>, Option<Minutes>),
    /// Critique.
    pub critical: (bool, bool),
    /// Minutes écoulées fin après − fin avant (0 si un côté est absent).
    pub finish_delta_min: i64,
}

impl ScheduleDiff {
    /// Diff vide d'un planning inchangé.
    #[must_use]
    pub fn unchanged(s: &Schedule) -> Self {
        Self {
            project_finish: (s.project_finish, s.project_finish),
            project_start: (s.project_start, s.project_start),
            critical_entered: Vec::new(),
            critical_left: Vec::new(),
            changed_total: 0,
            changed: Vec::new(),
            truncated: false,
        }
    }
}

/// Nombre de tâches du diff par défaut (`detail` : `diff_detail_max`).
pub const DEFAULT_DIFF_TASKS: usize = 10;

/// R-163 : diff des plannings avant/après, `changed` limité à `n`.
///
/// Jointure triée des `Schedule.tasks` par UID ; une tâche est changée si
/// son début ou sa fin planifiés diffèrent, ou si elle apparaît ou
/// disparaît. La criticité est suivie sur toutes les tâches. Tri : |Δ fin|
/// décroissant, ID d'après croissant (absent en dernier), UID.
pub(crate) fn diff_schedules(
    before_p: &Project,
    before: &Schedule,
    after_p: &Project,
    after: &Schedule,
    n: usize,
) -> ScheduleDiff {
    let mut d = ScheduleDiff {
        project_finish: (before.project_finish, after.project_finish),
        project_start: (before.project_start, after.project_start),
        critical_entered: Vec::new(),
        critical_left: Vec::new(),
        changed_total: 0,
        changed: Vec::new(),
        truncated: false,
    };
    let mut changed = Vec::new();
    for (uid, b, a) in Join::new(&before.tasks, &after.tasks) {
        let bs = b.and_then(scheduled);
        let as_ = a.and_then(scheduled);
        let crit = (bs.is_some_and(|s| s.critical), as_.is_some_and(|s| s.critical));
        match crit {
            (false, true) => d.critical_entered.push(uid),
            (true, false) => d.critical_left.push(uid),
            _ => {}
        }
        let start = (bs.map(|s| s.start), as_.map(|s| s.start));
        let finish = (bs.map(|s| s.finish), as_.map(|s| s.finish));
        let presence_changed = b.is_some() != a.is_some();
        if !presence_changed && start.0 == start.1 && finish.0 == finish.1 {
            continue;
        }
        let finish_delta_min = match finish {
            (Some(f0), Some(f1)) => f0.minutes_until(f1).get(),
            _ => 0,
        };
        changed.push(TaskChange {
            uid,
            id_before: b.and(before_p.task_id(uid)),
            id_after: a.and(after_p.task_id(uid)),
            start,
            finish,
            total_slack: (bs.map(|s| s.total_slack), as_.map(|s| s.total_slack)),
            critical: crit,
            finish_delta_min,
        });
    }
    changed.sort_by(compare);
    d.changed_total = changed.len();
    d.truncated = changed.len() > n;
    changed.truncate(n);
    d.changed = changed;
    d
}

/// Ordre de R-163.
fn compare(x: &TaskChange, y: &TaskChange) -> Ordering {
    y.finish_delta_min
        .unsigned_abs()
        .cmp(&x.finish_delta_min.unsigned_abs())
        .then_with(|| match (x.id_after, y.id_after) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        })
        .then_with(|| x.uid.cmp(&y.uid))
}

const fn scheduled(o: &TaskOutcome) -> Option<&TaskSchedule> {
    match o {
        TaskOutcome::Scheduled(s) => Some(s),
        TaskOutcome::Excluded { .. } => None,
    }
}

/// Jointure externe triée de deux tables par UID.
struct Join<'a> {
    left: Peekable<btree_map::Iter<'a, TaskUid, TaskOutcome>>,
    right: Peekable<btree_map::Iter<'a, TaskUid, TaskOutcome>>,
}

impl<'a> Join<'a> {
    fn new(
        left: &'a std::collections::BTreeMap<TaskUid, TaskOutcome>,
        right: &'a std::collections::BTreeMap<TaskUid, TaskOutcome>,
    ) -> Self {
        Self {
            left: left.iter().peekable(),
            right: right.iter().peekable(),
        }
    }
}

impl<'a> Iterator for Join<'a> {
    type Item = (TaskUid, Option<&'a TaskOutcome>, Option<&'a TaskOutcome>);

    fn next(&mut self) -> Option<Self::Item> {
        let side = match (self.left.peek(), self.right.peek()) {
            (None, None) => return None,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some(l), Some(r)) => l.0.cmp(r.0),
        };
        match side {
            Ordering::Less => self.left.next().map(|(k, v)| (*k, Some(v), None)),
            Ordering::Greater => self.right.next().map(|(k, v)| (*k, None, Some(v))),
            Ordering::Equal => {
                let l = self.left.next()?;
                let r = self.right.next();
                Some((*l.0, Some(l.1), r.map(|(_, v)| v)))
            }
        }
    }
}
