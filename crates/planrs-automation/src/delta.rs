//! Delta par entité entre deux projets : pré- et post-images des seules
//! entités changées (ADR-0012 D3, lot L2).

#![allow(
    dead_code,
    reason = "contrat J4 (L0) : appelé par Session et History (L2) ; attribut à retirer en L2"
)]

use planrs_core::calendar::Calendar;
use planrs_core::{
    Assignment, AssignmentUid, CalendarUid, Extras, ModelError, Moment, Project, ProjectInfo, ProjectOptions,
    Resource, ResourceUid, ScheduleAnchor, Task, TaskUid,
};

/// Delta d'un lot commité.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Delta {
    pub tasks: ArenaDelta<TaskUid, Task>,
    pub resources: ArenaDelta<ResourceUid, Resource>,
    pub assignments: ArenaDelta<AssignmentUid, Assignment>,
    pub calendars: ArenaDelta<CalendarUid, Calendar>,
    /// Blocs du projet hors arènes (avant, après), s'ils ont changé.
    pub header: Option<Box<(ProjectHeader, ProjectHeader)>>,
}

/// Delta d'une arène : `(uid, avant, après)` des entités changées, et
/// l'ordre des UID (avant, après) s'il a changé.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArenaDelta<K, T> {
    pub changed: Vec<(K, Option<T>, Option<T>)>,
    pub order: Option<(Vec<K>, Vec<K>)>,
}

impl<K, T> Default for ArenaDelta<K, T> {
    fn default() -> Self {
        Self {
            changed: Vec::new(),
            order: None,
        }
    }
}

/// Champs du projet hors arènes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProjectHeader {
    pub info: ProjectInfo,
    pub anchor: ScheduleAnchor,
    pub status_date: Option<Moment>,
    pub current_date: Moment,
    pub calendar: CalendarUid,
    pub options: ProjectOptions,
    pub summary_task: Option<Task>,
    pub extras: Extras,
}

/// Sens d'application d'un delta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Dir {
    /// Pré-images.
    Undo,
    /// Post-images.
    Redo,
}

/// Delta entre deux projets, par UID. Squelette L0 : vide, écrit en L2.
pub(crate) fn diff_projects(before: &Project, after: &Project) -> Delta {
    let _ = (before, after);
    Delta::default()
}

/// Applique un delta. Squelette L0 : sans effet, écrit en L2.
#[allow(
    clippy::unnecessary_wraps,
    reason = "contrat J4 (L0) : signature figée, corps écrit en L2"
)]
pub(crate) fn apply_delta(p: &mut Project, d: &Delta, dir: Dir) -> Result<(), ModelError> {
    let _ = (p, d, dir);
    Ok(())
}

/// Taille approchée d'un delta, en octets (budget R-164). Squelette L0.
pub(crate) fn approx_bytes(d: &Delta) -> usize {
    let _ = d;
    0
}
