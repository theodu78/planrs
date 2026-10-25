//! Colonnes de la table des tâches et leurs métadonnées.
//!
//! L'ordre des variantes de [`Column`] est l'ordre d'affichage : pas de tri ni
//! de réordonnancement dans les fondations (l'ordre des lignes est celui des
//! ID, l'ordre des colonnes celui de [`Column::ALL`]). Colonnes J7 (contrat
//! L0, ADR-0015) : % achevé, Noms ressources, Début et Fin de référence,
//! Variation de fin, masquées par défaut, à la fin ; leurs textes sont
//! remplis par le lot L6 (vides jusque-là).

use serde::{Deserialize, Serialize};

use crate::i18n::TextId;

/// Colonne de la table des tâches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Column {
    /// Identifiant de ligne (ID MS Project, position dans la liste).
    Id,
    /// Nom de la tâche, indenté selon le niveau hiérarchique (colonne du
    /// curseur par défaut).
    #[default]
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
    /// % achevé (J7, éditable au lot L6).
    PercentComplete,
    /// Noms des ressources affectées (J7, lecture seule ; N11).
    ResourceNames,
    /// Début de la ligne de base 0 (J7).
    BaselineStart,
    /// Fin de la ligne de base 0 (J7).
    BaselineFinish,
    /// Variation de fin par rapport à la ligne de base 0, en jours ouvrés
    /// signés (J7).
    FinishVariance,
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
    pub const ALL: [Self; 13] = [
        Self::Id,
        Self::Name,
        Self::Duration,
        Self::Start,
        Self::Finish,
        Self::Predecessors,
        Self::TotalSlack,
        Self::Critical,
        Self::PercentComplete,
        Self::ResourceNames,
        Self::BaselineStart,
        Self::BaselineFinish,
        Self::FinishVariance,
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
            Self::PercentComplete => 8,
            Self::ResourceNames => 9,
            Self::BaselineStart => 10,
            Self::BaselineFinish => 11,
            Self::FinishVariance => 12,
        }
    }

    /// Largeur par défaut, en points logiques (colonnes visibles : 642 au
    /// total).
    #[must_use]
    pub const fn default_width(self) -> u16 {
        match self {
            Self::Id => 40,
            Self::Name => 220,
            Self::Duration | Self::TotalSlack | Self::FinishVariance => 80,
            Self::Start | Self::Finish | Self::BaselineStart | Self::BaselineFinish => 96,
            Self::Predecessors => 110,
            Self::Critical | Self::PercentComplete => 60,
            Self::ResourceNames => 140,
        }
    }

    /// Visible par défaut : toutes sauf la marge totale, « Critique » et
    /// les colonnes J7.
    #[must_use]
    pub const fn default_visible(self) -> bool {
        !matches!(
            self,
            Self::TotalSlack
                | Self::Critical
                | Self::PercentComplete
                | Self::ResourceNames
                | Self::BaselineStart
                | Self::BaselineFinish
                | Self::FinishVariance
        )
    }

    /// La colonne peut-elle être masquée ? L'ID et le Nom restent toujours
    /// visibles (hypothèse d'affichage H-UI-12).
    #[must_use]
    pub const fn hideable(self) -> bool {
        !matches!(self, Self::Id | Self::Name)
    }

    /// Alignement du texte des cellules : nombres à droite (ID, durée,
    /// marge), indicateur « Critique » centré, le reste à gauche.
    #[must_use]
    pub const fn align(self) -> CellAlign {
        match self {
            Self::Id | Self::Duration | Self::TotalSlack | Self::PercentComplete | Self::FinishVariance => {
                CellAlign::Right
            }
            Self::Critical => CellAlign::Center,
            Self::Name
            | Self::Start
            | Self::Finish
            | Self::Predecessors
            | Self::ResourceNames
            | Self::BaselineStart
            | Self::BaselineFinish => CellAlign::Left,
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
            Self::PercentComplete => TextId::ColPercentComplete,
            Self::ResourceNames => TextId::ColResourceNames,
            Self::BaselineStart => TextId::ColBaselineStart,
            Self::BaselineFinish => TextId::ColBaselineFinish,
            Self::FinishVariance => TextId::ColFinishVariance,
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use std::collections::BTreeSet;

    use super::{CellAlign, Column, ColumnState};
    use crate::i18n::TextId;

    #[test]
    fn column_ordinal_matches_all() {
        assert_eq!(Column::COUNT, 13);
        for (index, column) in Column::ALL.iter().enumerate() {
            assert_eq!(column.ordinal(), index, "{column:?}");
        }
    }

    #[test]
    fn column_default_widths() {
        let widths: Vec<u16> = Column::ALL.iter().map(|c| c.default_width()).collect();
        assert_eq!(widths, [40, 220, 80, 96, 96, 110, 80, 60, 60, 140, 96, 96, 80]);
        for column in Column::ALL {
            let width = column.default_width();
            assert!((Column::MIN_WIDTH..=Column::MAX_WIDTH).contains(&width));
        }
    }

    #[test]
    fn column_default_visibility_and_hideable() {
        let hidden: Vec<Column> = Column::ALL.into_iter().filter(|c| !c.default_visible()).collect();
        assert_eq!(
            hidden,
            [
                Column::TotalSlack,
                Column::Critical,
                Column::PercentComplete,
                Column::ResourceNames,
                Column::BaselineStart,
                Column::BaselineFinish,
                Column::FinishVariance,
            ]
        );
        assert_eq!(Column::default(), Column::Name);
        assert!(!Column::Id.hideable());
        assert!(!Column::Name.hideable());
        assert!(Column::Duration.hideable());
        assert!(Column::Critical.hideable());
    }

    #[test]
    fn column_align() {
        assert_eq!(Column::Id.align(), CellAlign::Right);
        assert_eq!(Column::Duration.align(), CellAlign::Right);
        assert_eq!(Column::TotalSlack.align(), CellAlign::Right);
        assert_eq!(Column::Critical.align(), CellAlign::Center);
        for column in [Column::Name, Column::Start, Column::Finish, Column::Predecessors] {
            assert_eq!(column.align(), CellAlign::Left, "{column:?}");
        }
    }

    #[test]
    fn column_titles_distinct() {
        let titles: BTreeSet<TextId> = Column::ALL.iter().map(|c| c.title()).collect();
        assert_eq!(titles.len(), Column::COUNT);
        assert_eq!(Column::Name.title(), TextId::ColName);
    }

    #[test]
    fn column_state_default_for() {
        let state = ColumnState::default_for(Column::Critical);
        assert_eq!(
            state,
            ColumnState {
                column: Column::Critical,
                visible: false,
                width: 60
            }
        );
        let state = ColumnState::default_for(Column::Name);
        assert!(state.visible);
        assert_eq!(state.width, 220);
    }
}
