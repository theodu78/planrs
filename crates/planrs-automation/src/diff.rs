//! Diff de planning entre deux états (R-163, lot L2).

#![allow(
    dead_code,
    reason = "contrat J4 (L0) : appelé par Session::apply (L2) ; attribut à retirer en L2"
)]

use planrs_core::{Minutes, Moment, Project, Schedule, TaskId, TaskUid};
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

/// R-163 : diff des plannings avant/après, `changed` limité à `n`.
/// Squelette L0 : dates du projet seules, tâches à écrire en L2.
pub(crate) fn diff_schedules(
    before_p: &Project,
    before: &Schedule,
    after_p: &Project,
    after: &Schedule,
    n: usize,
) -> ScheduleDiff {
    let _ = (before_p, after_p, n);
    let mut d = ScheduleDiff::unchanged(after);
    d.project_finish.0 = before.project_finish;
    d.project_start.0 = before.project_start;
    d
}
