//! Valeurs calculées par MS Project, telles qu'enregistrées dans le fichier
//! (R-083). Elles ne vont jamais dans le `Project` : le harnais du corpus les
//! compare au moteur, qui les recalcule.

use std::collections::BTreeMap;

use planrs_core::{Minutes, Moment, TaskUid};

/// Dixièmes de minute, tels que MSPDI les écrit (`TotalSlack`, `FreeSlack`,
/// `LevelingDelay`). Exemple : 9 600 = 960 minutes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tenths(pub i64);

/// Planning enregistré.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordedSchedule {
    /// `StartDate` du projet.
    pub project_start: Option<Moment>,
    /// `FinishDate` du projet.
    pub project_finish: Option<Moment>,
    /// Une entrée par tâche lue, UID 0 compris.
    pub tasks: BTreeMap<TaskUid, RecordedTask>,
}

/// Valeurs enregistrées d'une tâche ; chaque champ est absent s'il manque
/// ou s'il est illisible (diagnostic).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecordedTask {
    /// `Start`.
    pub start: Option<Moment>,
    /// `Finish`.
    pub finish: Option<Moment>,
    /// `EarlyStart`.
    pub early_start: Option<Moment>,
    /// `EarlyFinish`.
    pub early_finish: Option<Moment>,
    /// `LateStart`.
    pub late_start: Option<Moment>,
    /// `LateFinish`.
    pub late_finish: Option<Moment>,
    /// `TotalSlack`, en dixièmes de minute.
    pub total_slack: Option<Tenths>,
    /// `FreeSlack`, en dixièmes de minute.
    pub free_slack: Option<Tenths>,
    /// `Critical`.
    pub critical: Option<bool>,
    /// `Summary`.
    pub summary: Option<bool>,
    /// `Duration` enregistrée (récapitulatives : valeur calculée).
    pub duration: Option<Minutes>,
    /// `LevelingDelay`, en dixièmes (J5, cause de divergence).
    pub leveling_delay: Option<Tenths>,
    /// Nombre d'affectations de la tâche (section `Assignments`, `TaskUID`).
    pub assignments: u32,
}
