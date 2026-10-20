//! Fenêtres modales (J7, ADR-0015 D6) : « Informations sur la tâche »
//! (lot L9), temps de travail, informations du projet, ligne de base,
//! replanifier le reste (lot L12), affectation des ressources (lot L13),
//! boîte d'un lien (lot L8).
//!
//! Une fenêtre à la fois, hébergée par `App` ; son brouillon est figé à
//! l'ouverture (`cache_key`) et « OK » produit un seul lot (une entrée
//! d'annulation). Aucune fenêtre n'est persistée.
//!
//! Contrat J7 (lot L0) : [`DialogState`] seulement.

use crate::action::DialogKind;

/// Fenêtre ouverte (état transitoire de la vue, jamais sérialisé).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialogState {
    /// Fenêtre.
    pub kind: DialogKind,
}
