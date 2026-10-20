//! Commandes `progress.*` (J6, ADR-0014 D3 à D5 ; R-216, R-217, R-219).
//! Contrat J6 (lot L0) ; exécution écrite par le lot L6.
//!
//! `progress.update` appelle `planrs_core::progress::edit::apply` avec les
//! dates planifiées de la session, puis écrit l'avancement, la durée et le
//! travail des affectations ; `progress.reschedule` (« replanifier le
//! travail restant », S33 « Update Project ») pose une reprise sur les
//! tâches en cours et une SNET sur les tâches non commencées en retard.
//!
//! Squelette L0 : refusées `command.not_yet_available` (jalon 6).

use planrs_core::Moment;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::exec::Ctx;
use super::{DurationJson, TaskRef};
use crate::error::CommandError;
use crate::json::{MomentSchema, PercentJson};

/// `progress.update` (R-216) : avancement d'une feuille. Champs absents :
/// inchangés. Conflits refusés (`param.conflict`, R-202) : `percent_complete`
/// avec `actual_duration` ; `percent_complete` < 100 avec `actual_finish` ;
/// `actual_finish` avec `remaining_duration` > 0 ; `percent_work_complete`
/// avec `actual_work`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgressUpdate {
    /// Tâche (feuille active).
    pub task: TaskRef,
    /// % achevé (durée) (R-201).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent_complete: Option<PercentJson>,
    /// Début réel (R-202).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<MomentSchema>")]
    pub actual_start: Option<Moment>,
    /// Fin réelle (R-202 : 100 %, durée mesurée).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<MomentSchema>")]
    pub actual_finish: Option<Moment>,
    /// Durée réelle (R-202).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_duration: Option<DurationJson>,
    /// Durée restante (R-202 : durée = réel + reste).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining_duration: Option<DurationJson>,
    /// % travail achevé (R-211).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent_work_complete: Option<PercentJson>,
    /// Travail réel de la tâche, réparti sur ses affectations (R-211).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_work: Option<DurationJson>,
    /// % physique (R-212, sans effet sur les dates).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_percent_complete: Option<PercentJson>,
}

/// `progress.reschedule` (R-217) : replanifie le travail restant après
/// `after`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProgressReschedule {
    /// Date après laquelle le reste est replanifié (défaut : date d'état
    /// effective, R-219).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<MomentSchema>")]
    pub after: Option<Moment>,
    /// Tâches concernées (défaut : toutes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tasks: Option<Vec<TaskRef>>,
}

/// `progress.update` : squelette L0.
pub(super) fn update(op: &ProgressUpdate, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(6))
}

/// `progress.reschedule` : squelette L0.
pub(super) fn reschedule(op: &ProgressReschedule, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(6))
}
