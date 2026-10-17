//! Colonnes de la table des tâches et leurs métadonnées.
//!
//! L'ordre des variantes de [`Column`] est l'ordre d'affichage : pas de tri ni
//! de réordonnancement dans les fondations (l'ordre des lignes est celui des
//! ID, l'ordre des colonnes celui de [`Column::ALL`]).

use serde::{Deserialize, Serialize};

use crate::i18n::TextId;

/// Colonne de la table des tâches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Column {
    /// Identifiant de ligne (ID MS Project, position dans la liste).
    Id,
    /// Nom de la tâche, indenté selon le niveau hiérarchique.
    Name,
    /// Durée au format d'affichage MS Project (« 5 jours »).
    Duration,
    /// Début calculé.
    Start,
    /// Fin calculée.
    Finish,
    /// Prédécesseurs (« 3;5DD+2 jours »).
    Predecessors,
    /// Marge totale, en jours.
    TotalSlack,
    /// Tâche critique (oui / non).
    Critical,
}

/// Alignement horizontal du texte d'une cellule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellAlign {
    /// Aligné à gauche.
    Left,
    /// Aligné à droite (nombres).
    Right,
    /// Centré.
    Center,
}

impl Column {
    /// Toutes les colonnes, dans l'ordre d'affichage.
    pub const ALL: [Self; 8] = [
        Self::Id,
        Self::Name,
        Self::Duration,
        Self::Start,
        Self::Finish,
        Self::Predecessors,
        Self::TotalSlack,
        Self::Critical,
    ];

    /// Nombre de colonnes (longueur de [`Column::ALL`]).
    pub const COUNT: usize = Self::ALL.len();

    /// Largeur minimale d'une colonne, en points logiques.
    pub const MIN_WIDTH: u16 = 24;

    /// Largeur maximale d'une colonne, en points logiques.
    pub const MAX_WIDTH: u16 = 2_000;

    /// Position dans [`Column::ALL`] (sert d'indice des textes pré-formatés).
    #[must_use]
    pub const fn ordinal(self) -> usize {
        match self {
            Self::Id => 0,
            Self::Name => 1,
            Self::Duration => 2,
            Self::Start => 3,
            Self::Finish => 4,
            Self::Predecessors => 5,
            Self::TotalSlack => 6,
            Self::Critical => 7,
        }
    }

    /// Largeur par défaut, en points logiques (colonnes visibles : 642 au
    /// total).
    #[must_use]
    pub const fn default_width(self) -> u16 {
        match self {
            Self::Id => 40,
            Self::Name => 220,
            Self::Duration | Self::TotalSlack => 80,
            Self::Start | Self::Finish => 96,
            Self::Predecessors => 110,
            Self::Critical => 64,
        }
    }

    /// Visible par défaut : toutes sauf la marge totale et « Critique ».
    #[must_use]
    pub const fn default_visible(self) -> bool {
        !matches!(self, Self::TotalSlack | Self::Critical)
    }

    /// Alignement du texte des cellules.
    #[must_use]
    pub const fn align(self) -> CellAlign {
        match self {
            Self::Id => CellAlign::Right,
            Self::Critical => CellAlign::Center,
            Self::Name
            | Self::Duration
            | Self::Start
            | Self::Finish
            | Self::Predecessors
            | Self::TotalSlack => CellAlign::Left,
        }
    }

    /// Identifiant du titre traduit de la colonne.
    #[must_use]
    pub const fn title(self) -> TextId {
        match self {
            Self::Id => TextId::ColId,
            Self::Name => TextId::ColName,
            Self::Duration => TextId::ColDuration,
            Self::Start => TextId::ColStart,
            Self::Finish => TextId::ColFinish,
            Self::Predecessors => TextId::ColPredecessors,
            Self::TotalSlack => TextId::ColTotalSlack,
            Self::Critical => TextId::ColCritical,
        }
    }
}

/// État persistant d'une colonne : visibilité et largeur (points entiers,
/// aucun flottant dans l'état sérialisé).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnState {
    /// Colonne concernée.
    pub column: Column,
    /// Affichée ou masquée.
    pub visible: bool,
    /// Largeur en points logiques.
    pub width: u16,
}

impl ColumnState {
    /// État par défaut d'une colonne.
    #[must_use]
    pub const fn default_for(column: Column) -> Self {
        Self {
            column,
            visible: column.default_visible(),
            width: column.default_width(),
        }
    }
}
