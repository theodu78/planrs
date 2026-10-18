//! Vues JSON partagées par la CLI (`--json`), le MCP et le canal
//! (ADR-0012 D1, lot L6). Toute date vient du `Schedule` (règle 3,
//! C-MCP-01).
//!
//! Contrat L0 : [`SummaryView`] seule (utilisée par le canal, L10) ; les
//! autres vues et la forme définitive sont écrites par L6.

use serde::Serialize;

use crate::json::DurationJson;

/// Résumé du projet (`project.summary`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SummaryView {
    /// Nom du projet.
    pub name: String,
    /// Début planifié (ISO local).
    pub start: String,
    /// Fin planifiée (ISO local).
    pub finish: String,
    /// Durée du projet.
    pub duration: DurationJson,
    /// Tâches (lignes vides exclues).
    pub tasks: usize,
    /// Récapitulatives.
    pub summaries: usize,
    /// Jalons.
    pub milestones: usize,
    /// Tâches critiques.
    pub critical: usize,
    /// Liens.
    pub links: usize,
    /// Diagnostics de gravité erreur.
    pub errors: usize,
    /// Avertissements.
    pub warnings: usize,
    /// Document modifié depuis l'enregistrement.
    pub dirty: bool,
    /// Entrées d'historique.
    pub history_len: usize,
    /// Chemin du fichier.
    pub path: Option<String>,
}
