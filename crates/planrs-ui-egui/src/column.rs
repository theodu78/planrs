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
use crate::state::SortKey;

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
    /// Mode de planification, « Manuelle » / « Automatique » (J8, lot L11,
    /// H-UI-36 ; lecture seule, masquée par défaut).
    Mode,
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
    pub const ALL: [Self; 14] = [
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
        Self::Mode,
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
            Self::Mode => 13,
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
            Self::Predecessors | Self::Mode => 110,
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
                | Self::Mode
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
            | Self::BaselineFinish
            | Self::Mode => CellAlign::Left,
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
            Self::Mode => TextId::ColMode,
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

/// Entrées ajoutées au menu de l'en-tête de colonne (H-UI-99, ADR-0018 D7 :
/// Ajuster la largeur, Trier par ordre croissant, décroissant) et
/// info-bulle de la poignée de bordure ; posées en L0, placées par le lot
/// L3 (`view::table::column_menu`, `view::table::resize_grip`).
pub const HEADER_MENU_TEXTS: [TextId; 4] = [
    TextId::MenuFitColumn,
    TextId::MenuSortAscending,
    TextId::MenuSortDescending,
    TextId::TipColumnResize,
];

/// Marge ajoutée à la largeur ajustée, en points : la troncature par « … »
/// (`grid::truncated_galley`) ne doit jamais jouer sur le texte le plus
/// large après un arrondi.
const FIT_SLACK: f32 = 1.0;

/// Largeur ajustée d'une colonne (H-UI-99, S88 : double-clic sur la bordure
/// d'en-tête) : la plus grande des largeurs de texte `texts` (retrait de
/// niveau de chaque ligne déjà compris), augmentée de `indent` (retrait
/// commun à toutes les lignes), et du titre `title`, plus les marges `pad`
/// de chaque côté, arrondie au point supérieur et bornée à
/// `[Column::MIN_WIDTH, Column::MAX_WIDTH]`. Les valeurs non finies ou
/// négatives sont ignorées ; `None` si aucune valeur n'est utilisable ou si
/// `pad` ou `indent` ne sont pas finis.
#[must_use]
pub fn fit_width(texts: impl Iterator<Item = f32>, title: f32, pad: f32, indent: f32) -> Option<u16> {
    if !pad.is_finite() || !indent.is_finite() {
        return None;
    }
    let usable = |w: &f32| w.is_finite() && *w >= 0.0;
    let widest = texts
        .filter(usable)
        .map(|w| w + indent.max(0.0))
        .chain(std::iter::once(title).filter(usable))
        .reduce(f32::max)?;
    let w = (widest + 2.0 * pad.max(0.0) + FIT_SLACK).ceil();
    if !w.is_finite() {
        return None;
    }
    let w = w.clamp(f32::from(Column::MIN_WIDTH), f32::from(Column::MAX_WIDTH));
    // Borné à [24, 2 000] et fini : la conversion est exacte.
    u16::try_from(crate::geom::f32_to_index(w)).ok()
}

/// Clé de tri de la vue des tâches nommée par la colonne (menu d'en-tête,
/// H-UI-99) : seules les clés de [`SortKey`] ; `None` pour une colonne qui
/// ne se trie pas (entrées « Trier » grisées).
#[must_use]
pub fn sort_key(column: Column) -> Option<SortKey> {
    SortKey::ALL.into_iter().find(|key| key.column() == column)
}

impl crate::App {
    /// Ajuste la largeur de `column` au contenu des lignes affichées et à
    /// son titre (`Action::FitColumn`, double-clic sur la bordure ou menu
    /// d'en-tête, H-UI-99) : galley de chaque texte du `RowModel` des lignes
    /// affichées dans la police de la table (Inter si le style la demande),
    /// retrait de niveau du Nom (un cran de plus avec les triangles de
    /// repli) et gras simulé des récapitulatives compris ; titre dans la
    /// police des titres ; marges de `Metrics`. La largeur passe par le
    /// chemin de `SetColumnWidth` (`UiState::set_column_width`, bornée) :
    /// une préférence d'affichage, hors de l'historique d'annulation.
    pub(crate) fn fit_column(&mut self, column: Column, ctx: &egui::Context) {
        let (screen, _) = self.screen_style(ctx.theme());
        let metrics = crate::view::Metrics::default();
        let title_font = egui::TextStyle::Body.resolve(&ctx.global_style());
        let mut font = title_font.clone();
        if screen.inter() {
            font.family = egui::FontFamily::Name(crate::theme::PLAN_FAMILY.into());
        }
        let title = self.texts.get(column.title());
        let outline = self.outline_triangles;
        let Some(rows) = self.visible_rows() else {
            return;
        };
        let width = ctx.fonts_mut(|fonts| {
            let mut measure = |text: &str, font: &egui::FontId| {
                fonts
                    .layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::PLACEHOLDER)
                    .size()
                    .x
            };
            let texts: Vec<f32> = (0..rows.len())
                .filter_map(|i| {
                    let d = crate::row_view::DisplayRow::at(i);
                    let row = rows.row(d)?;
                    let text = rows.text(d, column);
                    if text.is_empty() || row.kind == crate::rows::RowKind::Null {
                        return None;
                    }
                    let level = match column {
                        Column::Name if outline => row.level.saturating_add(1),
                        Column::Name => row.level,
                        _ => 0,
                    };
                    let indent = metrics.indent * f32::from(level.saturating_sub(1));
                    let bold = if row.kind == crate::rows::RowKind::Summary {
                        crate::grid::FAUX_BOLD_DX
                    } else {
                        0.0
                    };
                    Some(measure(text, &font) + indent + bold)
                })
                .collect();
            let title_w = measure(&title, &title_font);
            fit_width(texts.into_iter(), title_w, metrics.pad_x, 0.0)
        });
        if let Some(width) = width {
            self.state.set_column_width(column, width);
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

    /// H-UI-99 : plus large des textes (retrait commun ajouté) et du titre,
    /// marges des deux côtés, un point de jeu, arrondi au point supérieur,
    /// bornes ; valeurs non finies ou négatives ignorées.
    #[test]
    fn column_fit_width_bounds() {
        use super::fit_width;
        assert_eq!(
            fit_width([50.2, 80.0, 12.0].into_iter(), 30.0, 4.0, 0.0),
            Some(89)
        );
        assert_eq!(fit_width([50.0].into_iter(), 30.0, 4.0, 14.0), Some(73));
        // Titre plus large que le contenu.
        assert_eq!(fit_width([10.0].into_iter(), 60.0, 4.0, 0.0), Some(69));
        assert_eq!(fit_width(std::iter::empty(), 60.0, 4.0, 0.0), Some(69));
        // Bornes.
        assert_eq!(
            fit_width([1.0].into_iter(), 1.0, 0.0, 0.0),
            Some(Column::MIN_WIDTH)
        );
        assert_eq!(
            fit_width([1e9].into_iter(), 0.0, 4.0, 0.0),
            Some(Column::MAX_WIDTH)
        );
        // Valeurs non finies ou négatives ignorées.
        assert_eq!(
            fit_width(
                [f32::NAN, f32::INFINITY, -5.0, 40.0].into_iter(),
                f32::NAN,
                4.0,
                0.0
            ),
            Some(49)
        );
        assert_eq!(fit_width([f32::NAN].into_iter(), f32::NAN, 4.0, 0.0), None);
        assert_eq!(fit_width([40.0].into_iter(), 10.0, f32::NAN, 0.0), None);
        assert_eq!(fit_width([40.0].into_iter(), 10.0, 4.0, f32::INFINITY), None);
        assert_eq!(fit_width([f32::MAX].into_iter(), 0.0, f32::MAX, 0.0), None);
    }

    /// H-UI-99 : les colonnes qui nomment une clé de `SortKey` se trient,
    /// les autres non.
    #[test]
    fn column_sort_key() {
        use super::sort_key;
        use crate::state::SortKey;
        for key in SortKey::ALL {
            assert_eq!(sort_key(key.column()), Some(key));
        }
        for column in [
            Column::Predecessors,
            Column::Critical,
            Column::ResourceNames,
            Column::BaselineStart,
            Column::BaselineFinish,
            Column::FinishVariance,
            Column::Mode,
        ] {
            assert_eq!(sort_key(column), None, "{column:?}");
        }
    }

    #[test]
    fn column_ordinal_matches_all() {
        assert_eq!(Column::COUNT, 14);
        for (index, column) in Column::ALL.iter().enumerate() {
            assert_eq!(column.ordinal(), index, "{column:?}");
        }
    }

    #[test]
    fn column_default_widths() {
        let widths: Vec<u16> = Column::ALL.iter().map(|c| c.default_width()).collect();
        assert_eq!(
            widths,
            [40, 220, 80, 96, 96, 110, 80, 60, 60, 140, 96, 96, 80, 110]
        );
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
                Column::Mode,
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
