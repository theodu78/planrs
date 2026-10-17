//! État d'interface sérialisable (`UiState`), séparé du modèle.
//!
//! `UiState` regroupe les préférences et la position de lecture, en JSON
//! (`serde_json`), sans aucune donnée du projet ni aucun flottant : le JSON est
//! déterministe et `UiState: Eq`. Lecture tolérante : champ absent → valeur
//! par défaut, champ inconnu ignoré (pas de `deny_unknown_fields`, une
//! version plus récente a pu écrire davantage).

use serde::{Deserialize, Serialize};

use crate::column::{Column, ColumnState};
use crate::timescale::TimeScaleZoom;

/// Version du format JSON. Elle s'incrémente quand le sens d'un champ change
/// ou qu'un champ est retiré ou renommé ; ajouter un champ avec valeur par
/// défaut ne l'incrémente pas.
pub const UI_STATE_VERSION: u32 = 1;

/// Clé de stockage eframe (desktop : fichier de `storage_dir`, web :
/// `localStorage`). Contient le nom provisoire : listée dans `docs/rename.md`.
pub const STATE_KEY: &str = "planrs.ui_state";

/// Largeur minimale de la table, en points logiques.
pub const TABLE_WIDTH_MIN: u16 = 120;

/// Largeur maximale de la table, en points logiques.
pub const TABLE_WIDTH_MAX: u16 = 4_000;

/// Largeur par défaut de la table : somme des colonnes visibles par défaut.
pub const TABLE_WIDTH_DEFAULT: u16 = 642;

/// Préférences d'interface persistées.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiState {
    /// Version du format ([`UI_STATE_VERSION`]).
    pub version: u32,
    /// Langue demandée.
    pub language: LanguagePref,
    /// Thème demandé.
    pub theme: ThemePref,
    /// Dernier zoom choisi (écrasé par l'ajustement à chaque ouverture).
    pub zoom: TimeScaleZoom,
    /// Une entrée par colonne, dans l'ordre de [`Column::ALL`].
    pub columns: Vec<ColumnState>,
    /// Largeur de la table, en points logiques.
    pub table_width: u16,
    /// Panneau des diagnostics ouvert.
    pub diagnostics_open: bool,
}

/// Langue demandée par l'utilisateur.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguagePref {
    /// Langue du système (repli : français).
    #[default]
    System,
    /// Français.
    Fr,
    /// Anglais.
    En,
}

/// Thème demandé par l'utilisateur.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePref {
    /// Thème du système.
    #[default]
    System,
    /// Clair.
    Light,
    /// Sombre.
    Dark,
}

/// Avertissement de lecture de l'état : affiché une fois, jamais bloquant.
#[derive(Debug, thiserror::Error)]
pub enum StateLoadError {
    /// JSON illisible : l'état par défaut est utilisé.
    #[error("JSON illisible : {0}")]
    Json(#[from] serde_json::Error),
    /// État écrit par une version plus récente : lu quand même.
    #[error("version {found} plus récente que {supported}")]
    TooNew {
        /// Version trouvée.
        found: u32,
        /// Version la plus récente comprise.
        supported: u32,
    },
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            version: UI_STATE_VERSION,
            language: LanguagePref::default(),
            theme: ThemePref::default(),
            zoom: TimeScaleZoom::default(),
            columns: Column::ALL.iter().map(|&c| ColumnState::default_for(c)).collect(),
            table_width: TABLE_WIDTH_DEFAULT,
            diagnostics_open: false,
        }
    }
}

impl UiState {
    /// Lecture tolérante : jamais d'échec. Rend l'état normalisé et, s'il y a
    /// lieu, l'avertissement à afficher une fois.
    #[must_use]
    pub fn from_json_lenient(json: &str) -> (Self, Option<StateLoadError>) {
        match serde_json::from_str::<Self>(json) {
            Ok(state) => {
                let warning = (state.version > UI_STATE_VERSION).then_some(StateLoadError::TooNew {
                    found: state.version,
                    supported: UI_STATE_VERSION,
                });
                (state.normalized(), warning)
            }
            Err(error) => (Self::default(), Some(StateLoadError::Json(error))),
        }
    }

    /// JSON compact déterministe (ordre des champs fixe, aucune map).
    ///
    /// # Errors
    ///
    /// Erreur de sérialisation de `serde_json` (non attendue pour ce type).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Bornes appliquées, colonnes complétées et dédoublonnées dans l'ordre
    /// de [`Column::ALL`], version ramenée à [`UI_STATE_VERSION`].
    #[must_use]
    pub fn normalized(mut self) -> Self {
        self.version = UI_STATE_VERSION;
        self.table_width = self.table_width.clamp(TABLE_WIDTH_MIN, TABLE_WIDTH_MAX);
        let columns = Column::ALL
            .iter()
            .map(|&column| {
                self.columns
                    .iter()
                    .find(|state| state.column == column)
                    .map_or_else(
                        || ColumnState::default_for(column),
                        |state| ColumnState {
                            column,
                            visible: state.visible,
                            width: state.width.clamp(Column::MIN_WIDTH, Column::MAX_WIDTH),
                        },
                    )
            })
            .collect();
        self.columns = columns;
        self
    }

    /// Colonnes visibles, dans l'ordre d'affichage.
    pub fn visible_columns(&self) -> impl Iterator<Item = ColumnState> + '_ {
        self.columns.iter().copied().filter(|state| state.visible)
    }
}
