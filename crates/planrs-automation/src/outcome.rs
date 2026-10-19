//! Résultat d'un lot appliqué (contrat J4, ADR-0012) : partagé par L2
//! (pipeline), L4 et L5 (effets de bord), L6 (vues).

use std::collections::BTreeMap;

use planrs_core::{AssignmentUid, ConstraintKind, Diagnostic, Subject, TaskId, TaskUid};
use serde::Serialize;

use crate::diff::ScheduleDiff;

/// Résultat de `Session::apply`, `undo` ou `redo`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ApplyOutcome {
    /// Vrai si l'état de la session a changé (faux en `DryRun` ou lot vide).
    pub committed: bool,
    /// Étiquettes `$x` → UID créées par le lot (R-150).
    pub labels: BTreeMap<String, TaskUid>,
    /// Tâches créées, dans l'ordre.
    pub created: Vec<TaskUid>,
    /// Diff de planning (R-163).
    pub diff: ScheduleDiff,
    /// Avertissements et informations nouveaux (R-160), triés.
    pub warnings: Vec<Diagnostic>,
    /// Effets de bord, dans l'ordre.
    pub side_effects: Vec<SideEffect>,
    /// Nombre d'entrées de l'historique après l'opération.
    pub history_len: usize,
}

/// Effet de bord d'une commande, signalé à l'appelant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffect {
    /// Lien retiré par une suppression (R-155).
    LinkRemoved {
        /// Prédécesseur.
        pred: TaskUid,
        /// Successeur.
        succ: TaskUid,
    },
    /// Affectation retirée par une suppression (R-155).
    AssignmentRemoved(AssignmentUid),
    /// Clés d'`Extras` jetées (R-161).
    ExtrasDropped {
        /// Entité.
        subject: Subject,
        /// Clés jetées.
        keys: Vec<String>,
    },
    /// ID renumérotés (R-150).
    IdsRenumbered {
        /// Premier ID changé.
        from: TaskId,
        /// Nombre de tâches renumérotées.
        count: usize,
    },
    /// Valeurs relues du fichier jetées au premier commit (R-162).
    RecordedDiscarded,
    /// Tâche supprimée (R-155).
    TaskDeleted(TaskUid),
    /// Contrainte posée par une date saisie (R-158).
    ConstraintSet {
        /// Tâche.
        uid: TaskUid,
        /// Type posé.
        kind: ConstraintKind,
    },
}
