//! Commande `baseline.set` (J6, ADR-0014 D6 ; R-227). Contrat J6 (lot
//! L0) ; exécution écrite par le lot L6 sur `planrs_core::baseline`.
//!
//! Squelette L0 : refusée `command.not_yet_available` (jalon 6).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::TaskRef;
use super::exec::Ctx;
use crate::error::CommandError;
use crate::json::RollupJson;

/// `baseline.set` (R-227) : enregistre la ligne de base `number` (0 à 10,
/// défaut 0) sur le projet entier ou sur `tasks`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BaselineSet {
    /// Numéro de la ligne (0 à 10 ; défaut 0).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(max = 10))]
    pub number: Option<u8>,
    /// Tâches sélectionnées (défaut : projet entier).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tasks: Option<Vec<TaskRef>>,
    /// Cumul vers les récapitulatives (défaut : aucun ; demande une
    /// sélection).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollup: Option<RollupJson>,
    /// Remplacer une ligne existante (sinon `baseline.exists`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub overwrite: bool,
}

/// `baseline.set` : squelette L0.
pub(super) fn set(op: &BaselineSet, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(6))
}
