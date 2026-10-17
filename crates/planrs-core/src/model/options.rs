//! Options du projet (ADR-0008, § 5.2, R-032, R-033).
//!
//! Deux séries de valeurs par défaut :
//! - celles de l'application, pour un projet neuf : [`ProjectOptions::default`] ;
//! - celles du schéma MSPDI, pour un élément absent à l'import
//!   (`DefaultTaskType` = durée fixe, `NewTasksEffortDriven` = vrai, S2) :
//!   appliquées par `planrs-mspdi` (J3), avec leur propre règle.

use serde::{Deserialize, Serialize};

use crate::duration::{ConversionFactors, DurationUnit};
use crate::error::TimeError;
use crate::model::task::TaskType;
use crate::time::{Day, Moment, TimeOfDay, Weekday};

/// Options du projet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectOptions {
    /// Facteurs de conversion des unités (R-006).
    pub factors: ConversionFactors,
    /// Heure de début par défaut d'une date saisie sans heure (R-033).
    pub default_start_time: TimeOfDay,
    /// Heure de fin par défaut d'une date saisie sans heure (R-033).
    pub default_finish_time: TimeOfDay,
    /// Unité de saisie des durées.
    pub default_duration_unit: DurationUnit,
    /// Unité de saisie du travail.
    pub default_work_unit: DurationUnit,
    /// Type des nouvelles tâches.
    pub default_task_type: TaskType,
    /// Nouvelles tâches pilotées par l'effort.
    pub new_tasks_effort_driven: bool,
    /// Durée des nouvelles tâches estimée (« 1j? »).
    pub new_tasks_estimated: bool,
    /// Premier jour de la semaine (affichage seulement).
    pub week_start_day: Weekday,
    /// Respecter les contraintes plutôt que les liens.
    pub honor_constraints: bool,
}

impl Default for ProjectOptions {
    /// Valeurs de l'application pour un projet neuf (R-032) : 480 / 2 400 /
    /// 20 (S8) ; 08:00 et 17:00 (S8, S9) ; durées en jours (S10), travail en
    /// heures ; unités fixes, sans pilotage par l'effort (hypothèse) ;
    /// estimées (S14) ; semaine commençant le lundi (hypothèse, version
    /// française) ; contraintes respectées (S2).
    fn default() -> Self {
        Self {
            factors: ConversionFactors::default(),
            default_start_time: TimeOfDay::hm(8, 0).unwrap_or(TimeOfDay::MIDNIGHT),
            default_finish_time: TimeOfDay::hm(17, 0).unwrap_or(TimeOfDay::END_OF_DAY),
            default_duration_unit: DurationUnit::Day,
            default_work_unit: DurationUnit::Hour,
            default_task_type: TaskType::FixedUnits,
            new_tasks_effort_driven: false,
            new_tasks_estimated: true,
            week_start_day: Weekday::Monday,
            honor_constraints: true,
        }
    }
}

impl ProjectOptions {
    /// R-033 : une date de début saisie sans heure reçoit l'heure de début par
    /// défaut. L'alignement sur le calendrier se fait ensuite au placement
    /// (R-030).
    ///
    /// # Errors
    ///
    /// [`TimeError`] si l'instant sort du domaine (impossible : un jour du
    /// domaine à une heure de 00:00 à 24:00 y reste).
    pub fn start_of(&self, day: Day) -> Result<Moment, TimeError> {
        Moment::at(day, self.default_start_time)
    }

    /// R-033 : une date de fin saisie sans heure reçoit l'heure de fin par
    /// défaut.
    ///
    /// # Errors
    ///
    /// [`TimeError`] si l'instant sort du domaine (impossible, voir
    /// [`ProjectOptions::start_of`]).
    pub fn finish_of(&self, day: Day) -> Result<Moment, TimeError> {
        Moment::at(day, self.default_finish_time)
    }
}
