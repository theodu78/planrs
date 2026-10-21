//! Fenêtres modales (J7, ADR-0015 D6) : « Informations sur la tâche »
//! (lot L9), temps de travail, informations du projet, ligne de base,
//! replanifier le reste (lot L12), affectation des ressources (lot L13),
//! boîte d'un lien (lot L8).
//!
//! Une fenêtre à la fois, hébergée par `App` ; son brouillon est figé à
//! l'ouverture (`cache_key`) et « OK » produit un seul lot (une entrée
//! d'annulation). Aucune fenêtre n'est persistée. Pendant qu'une fenêtre
//! est ouverte, aucun raccourci de `KEYMAP` n'est lu et les touches non
//! prises par la fenêtre sont retirées de l'image (rien n'atteint la
//! table, lot L9).

pub mod link;
pub mod task_info;

use crate::action::DialogKind;

/// Fenêtre ouverte (état transitoire de la vue, jamais sérialisé).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialogState {
    /// Fenêtre.
    pub kind: DialogKind,
    /// Contenu propre à la fenêtre (brouillon).
    pub body: DialogBody,
}

/// Contenu d'une fenêtre ouverte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DialogBody {
    /// Fenêtre sans contenu propre (pas encore câblée par son lot).
    Empty,
    /// « Informations sur la tâche » (lot L9).
    TaskInfo(Box<task_info::TaskInfoState>),
    /// Boîte « Dépendance » (lot L8, H-UI-21).
    Link(Box<link::LinkState>),
}
