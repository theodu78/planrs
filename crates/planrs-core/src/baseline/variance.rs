//! Écarts à une ligne de base (R-224, R-225 ; lot L4).
//!
//! Squelette L0 : écarts nuls, sans ligne.

use crate::calendar::CalendarSet;
use crate::duration::Minutes;
use crate::ids::{AssignmentUid, BaselineNumber, ResourceUid, TaskUid};
use crate::model::{Project, TaskBaseline};
use crate::schedule::Schedule;

/// Écarts signés d'une tâche, en minutes ouvrées de son échelle de mesure
/// (R-113) : valeur − ligne de base.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Variance {
    /// Écart de début (R-224) ; 0 sans ligne.
    pub start: Minutes,
    /// Écart de fin (R-224) ; 0 sans ligne.
    pub finish: Minutes,
    /// Écart de durée (R-225).
    pub duration: Minutes,
    /// Écart de travail (R-225) ; sans ligne : le travail (O43).
    pub work: Minutes,
}

/// Écarts d'une tâche.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskVariance {
    /// Tâche (UID 0 : projet).
    pub uid: TaskUid,
    /// Ligne de base de référence ; `None` si la tâche n'en a pas.
    pub baseline: Option<TaskBaseline>,
    /// Écarts.
    pub variance: Variance,
}

/// Écarts du projet à la ligne `number`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variances {
    /// Numéro de la ligne de référence.
    pub number: BaselineNumber,
    /// Projet (UID 0).
    pub project: TaskVariance,
    /// Tâches planifiées, dans l'ordre des UID.
    pub tasks: Vec<TaskVariance>,
    /// Écart de travail des ressources (R-225).
    pub resources: Vec<(ResourceUid, Minutes)>,
    /// Écart de travail des affectations (R-225).
    pub assignments: Vec<(AssignmentUid, Minutes)>,
}

/// Écarts du planning `s` de `p` à la ligne `number` (R-224, R-225).
///
/// Squelette L0 : écarts nuls, aucune ligne lue.
#[must_use]
pub fn variance(p: &Project, s: &Schedule, set: &CalendarSet, number: BaselineNumber) -> Variances {
    let _ = (p, s, set);
    Variances {
        number,
        project: TaskVariance {
            uid: TaskUid::PROJECT_SUMMARY,
            baseline: None,
            variance: Variance::default(),
        },
        tasks: Vec::new(),
        resources: Vec::new(),
        assignments: Vec::new(),
    }
}
