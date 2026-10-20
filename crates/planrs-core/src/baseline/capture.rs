//! Capture d'une ligne de base (R-220 à R-223 ; lot L4).
//!
//! Squelette L0 : [`capture`] contrôle la portée (tâches inconnues, cumul
//! sans sélection) et rend une capture vide ; [`apply`] n'écrit rien.

use std::collections::BTreeMap;

use crate::diagnostic::Subject;
use crate::ids::{AssignmentUid, BaselineNumber, ResourceUid, TaskUid};
use crate::model::{AssignmentBaseline, Project, ResourceBaseline, TaskBaseline};
use crate::schedule::Schedule;

/// Portée d'une capture (R-221).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Scope {
    /// Projet entier : toutes les tâches planifiées, leurs affectations,
    /// toutes les ressources.
    #[default]
    Project,
    /// Ces tâches et leurs affectations ; UID 0 seulement s'il est cité.
    Tasks(Vec<TaskUid>),
}

/// Cumul vers les récapitulatives (R-222 ; S58 « Roll up baselines »).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Rollup {
    /// Aucun cumul.
    #[default]
    None,
    /// « To all summary tasks » : chaque ancêtre d'une tâche de la portée.
    ToAllSummaries,
    /// « From subtasks into selected summary task(s) » : seules les
    /// récapitulatives de la sélection.
    FromSubtasks,
}

/// Demande de capture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaselineSpec {
    /// Numéro de la ligne (0 à 10).
    pub number: BaselineNumber,
    /// Portée.
    pub scope: Scope,
    /// Cumul.
    pub rollup: Rollup,
    /// Remplacer une ligne existante (R-223).
    pub overwrite: bool,
}

/// Lignes capturées, à écrire par [`apply`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BaselineEdit {
    /// Lignes des tâches (récapitulatives et UID 0 compris).
    pub tasks: BTreeMap<TaskUid, TaskBaseline>,
    /// Lignes des affectations.
    pub assignments: BTreeMap<AssignmentUid, AssignmentBaseline>,
    /// Lignes des ressources.
    pub resources: BTreeMap<ResourceUid, ResourceBaseline>,
    /// Tâches de la portée sautées (lignes vides, exclues du calcul ;
    /// `baseline.task_skipped`).
    pub skipped: Vec<TaskUid>,
    /// Un coût stocké a été recopié dans une ligne posée : planrs ne
    /// recalcule pas les coûts (`baseline.cost_not_computed`, arbitrage du
    /// 2026-10-20 de l'ADR-0014 : recopié, jamais effacé).
    pub cost_copied: bool,
}

/// Capture refusée.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum BaselineError {
    /// Ligne déjà présente sur `count` entités de la portée, sans
    /// `overwrite` (`baseline.exists`, R-223).
    #[error("la ligne de base {} existe déjà sur {count} élément(s)", .number.get())]
    Exists {
        /// Numéro.
        number: BaselineNumber,
        /// Entités qui l'ont déjà.
        count: u32,
    },
    /// Tâche de la portée inexistante (`task.not_found`).
    #[error("aucune tâche d'UID {0}")]
    UnknownTask(TaskUid),
    /// Cumul demandé sans sélection (`baseline.rollup_needs_selection`,
    /// R-222).
    #[error("le cumul vers les récapitulatives demande une sélection de tâches")]
    RollupNeedsSelection,
}

/// Capture la ligne `spec.number` sur la portée de `spec`, à partir du
/// planning `s` de `p` (R-220 à R-223).
///
/// # Errors
///
/// [`BaselineError`] ; squelette L0 : [`BaselineError::UnknownTask`] et
/// [`BaselineError::RollupNeedsSelection`] seulement.
pub fn capture(p: &Project, s: &Schedule, spec: &BaselineSpec) -> Result<BaselineEdit, BaselineError> {
    let _ = s;
    match &spec.scope {
        Scope::Project if spec.rollup != Rollup::None => return Err(BaselineError::RollupNeedsSelection),
        Scope::Project => {}
        Scope::Tasks(uids) => {
            if let Some(uid) = uids
                .iter()
                .find(|u| **u != TaskUid::PROJECT_SUMMARY && p.tasks.get(**u).is_none())
            {
                return Err(BaselineError::UnknownTask(*uid));
            }
        }
    }
    Ok(BaselineEdit::default())
}

/// Écrit la capture `edit` comme ligne `number` de `p` ; rend, par entité
/// remplacée, les clés d'`Extras` de portée `Baseline#n/` jetées (R-223,
/// extension de R-161).
///
/// Squelette L0 : n'écrit rien.
pub fn apply(p: &mut Project, number: BaselineNumber, edit: BaselineEdit) -> Vec<(Subject, Vec<String>)> {
    let _ = (p, number, edit);
    Vec::new()
}
