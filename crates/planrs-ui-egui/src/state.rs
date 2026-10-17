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
    ///
    /// - JSON illisible (ou d'une forme inattendue) : état par défaut et
    ///   [`StateLoadError::Json`] ;
    /// - `version` supérieure à [`UI_STATE_VERSION`] : état par défaut et
    ///   [`StateLoadError::TooNew`] (le sens des champs a pu changer) ;
    /// - champ absent : valeur par défaut ; champ inconnu : ignoré.
    #[must_use]
    pub fn from_json_lenient(json: &str) -> (Self, Option<StateLoadError>) {
        let value = match serde_json::from_str::<serde_json::Value>(json) {
            Ok(value) => value,
            Err(error) => return (Self::default(), Some(StateLoadError::Json(error))),
        };
        // serde accepte aussi un tableau pour une structure : seul un objet
        // est un état valide.
        if !value.is_object() {
            let error = <serde_json::Error as serde::de::Error>::custom("objet JSON attendu");
            return (Self::default(), Some(StateLoadError::Json(error)));
        }
        // La version est lue avant le reste : un état plus récent peut avoir
        // changé le type d'un champ connu, ce qui ferait échouer la lecture.
        if let Some(found) = value.get("version").and_then(serde_json::Value::as_u64)
            && found > u64::from(UI_STATE_VERSION)
        {
            let found = u32::try_from(found).unwrap_or(u32::MAX);
            return (
                Self::default(),
                Some(StateLoadError::TooNew {
                    found,
                    supported: UI_STATE_VERSION,
                }),
            );
        }
        match serde_json::from_value::<Self>(value) {
            Ok(state) => (state.normalized(), None),
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
    /// de [`Column::ALL`] (la première occurrence d'une colonne l'emporte),
    /// largeurs bornées à `[Column::MIN_WIDTH, Column::MAX_WIDTH]`, largeur
    /// de table bornée à `[TABLE_WIDTH_MIN, TABLE_WIDTH_MAX]`, ID et Nom
    /// forcés visibles, version ramenée à [`UI_STATE_VERSION`].
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
                            visible: state.visible || !column.hideable(),
                            width: state.width.clamp(Column::MIN_WIDTH, Column::MAX_WIDTH),
                        },
                    )
            })
            .collect();
        self.columns = columns;
        self
    }

    /// État d'une colonne, s'il est présent.
    #[must_use]
    pub fn column(&self, column: Column) -> Option<&ColumnState> {
        self.columns.iter().find(|state| state.column == column)
    }

    /// Colonnes visibles, dans l'ordre d'affichage.
    pub fn visible_columns(&self) -> impl Iterator<Item = ColumnState> + '_ {
        self.columns.iter().copied().filter(|state| state.visible)
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
    use super::{
        LanguagePref, StateLoadError, TABLE_WIDTH_DEFAULT, TABLE_WIDTH_MAX, TABLE_WIDTH_MIN, ThemePref,
        UI_STATE_VERSION, UiState,
    };
    use crate::column::{Column, ColumnState};
    use crate::timescale::TimeScaleZoom;

    /// JSON de référence de `UiState::default()` (format v1).
    const REFERENCE_V1: &str = include_str!("../tests/fixtures/ui_state_v1.json");

    #[test]
    fn ui_state_default_json_reference() {
        let json = UiState::default().to_json().unwrap();
        assert_eq!(
            json,
            REFERENCE_V1.trim_end(),
            "le format JSON de UiState a changé : décider s'il faut incrémenter UI_STATE_VERSION"
        );
        let (state, warning) = UiState::from_json_lenient(REFERENCE_V1);
        assert!(warning.is_none(), "{warning:?}");
        assert_eq!(state, UiState::default());
    }

    #[test]
    fn ui_state_default_values() {
        let state = UiState::default();
        assert_eq!(state.version, UI_STATE_VERSION);
        assert_eq!(state.language, LanguagePref::System);
        assert_eq!(state.theme, ThemePref::System);
        assert_eq!(state.zoom, TimeScaleZoom::Week);
        assert!(!state.diagnostics_open);
        let visible_width: u16 = state.visible_columns().map(|c| c.width).sum();
        assert_eq!(visible_width, TABLE_WIDTH_DEFAULT);
        assert_eq!(state.table_width, TABLE_WIDTH_DEFAULT);
        assert_eq!(state.clone().normalized(), state);
    }

    #[test]
    fn ui_state_garbage_is_default() {
        for garbage in [
            "",
            "{",
            "not json",
            "null",
            "42",
            "[]",
            "\"x\"",
            "{\"table_width\": -5}",
        ] {
            let (state, warning) = UiState::from_json_lenient(garbage);
            assert_eq!(state, UiState::default(), "{garbage:?}");
            assert!(
                matches!(warning, Some(StateLoadError::Json(_))),
                "{garbage:?} : {warning:?}"
            );
        }
    }

    #[test]
    fn ui_state_unknown_field_ignored() {
        let json = r#"{"version":1,"theme":"dark","future_field":{"a":[1,2]},"zoom":"month"}"#;
        let (state, warning) = UiState::from_json_lenient(json);
        assert!(warning.is_none(), "{warning:?}");
        assert_eq!(state.theme, ThemePref::Dark);
        assert_eq!(state.zoom, TimeScaleZoom::Month);
        // Champs absents : valeurs par défaut.
        assert_eq!(state.language, LanguagePref::System);
        assert_eq!(state.columns, UiState::default().columns);
        // Objet vide : état par défaut, sans avertissement.
        let (state, warning) = UiState::from_json_lenient("{}");
        assert!(warning.is_none());
        assert_eq!(state, UiState::default());
    }

    #[test]
    fn ui_state_too_new_warns() {
        let json = r#"{"version":2,"theme":"dark","zoom":{"px_per_day":3}}"#;
        let (state, warning) = UiState::from_json_lenient(json);
        assert_eq!(state, UiState::default());
        match warning {
            Some(StateLoadError::TooNew { found, supported }) => {
                assert_eq!(found, 2);
                assert_eq!(supported, UI_STATE_VERSION);
            }
            other => panic!("attendu TooNew, obtenu {other:?}"),
        }
        let (_, warning) = UiState::from_json_lenient(r#"{"version":99999999999}"#);
        assert!(matches!(
            warning,
            Some(StateLoadError::TooNew { found: u32::MAX, .. })
        ));
    }

    #[test]
    fn ui_state_normalized_bounds() {
        let state = UiState {
            version: 0,
            table_width: 1,
            columns: vec![
                ColumnState {
                    column: Column::Critical,
                    visible: true,
                    width: 0,
                },
                ColumnState {
                    column: Column::Id,
                    visible: false,
                    width: 9_999,
                },
                ColumnState {
                    column: Column::Critical,
                    visible: false,
                    width: 300,
                },
                ColumnState {
                    column: Column::Name,
                    visible: false,
                    width: 100,
                },
                ColumnState {
                    column: Column::Duration,
                    visible: false,
                    width: 50,
                },
            ],
            ..UiState::default()
        }
        .normalized();
        assert_eq!(state.version, UI_STATE_VERSION);
        assert_eq!(state.table_width, TABLE_WIDTH_MIN);
        let order: Vec<Column> = state.columns.iter().map(|c| c.column).collect();
        assert_eq!(order, Column::ALL);
        // Première occurrence retenue, largeur bornée.
        assert_eq!(
            state.column(Column::Critical),
            Some(&ColumnState {
                column: Column::Critical,
                visible: true,
                width: Column::MIN_WIDTH
            })
        );
        // ID et Nom toujours visibles.
        assert_eq!(
            state.column(Column::Id),
            Some(&ColumnState {
                column: Column::Id,
                visible: true,
                width: Column::MAX_WIDTH
            })
        );
        assert!(state.column(Column::Name).unwrap().visible);
        // Masquage respecté pour une colonne masquable.
        assert!(!state.column(Column::Duration).unwrap().visible);
        // Colonnes manquantes : défauts.
        assert_eq!(
            state.column(Column::Start),
            Some(&ColumnState::default_for(Column::Start))
        );
        let visible: Vec<Column> = state.visible_columns().map(|c| c.column).collect();
        assert_eq!(
            visible,
            [
                Column::Id,
                Column::Name,
                Column::Start,
                Column::Finish,
                Column::Predecessors,
                Column::Critical
            ]
        );

        let wide = UiState {
            table_width: u16::MAX,
            ..UiState::default()
        }
        .normalized();
        assert_eq!(wide.table_width, TABLE_WIDTH_MAX);

        // La lecture tolérante normalise aussi.
        let (read, warning) = UiState::from_json_lenient(r#"{"table_width":0,"columns":[]}"#);
        assert!(warning.is_none());
        assert_eq!(read.table_width, TABLE_WIDTH_MIN);
        assert_eq!(read.columns, UiState::default().columns);
    }

    #[test]
    fn ui_state_roundtrip() {
        let mut state = UiState {
            language: LanguagePref::En,
            theme: ThemePref::Light,
            zoom: TimeScaleZoom::Quarter,
            table_width: 800,
            diagnostics_open: true,
            ..UiState::default()
        };
        if let Some(slack) = state.columns.iter_mut().find(|c| c.column == Column::TotalSlack) {
            slack.visible = true;
            slack.width = 123;
        }
        let json = state.to_json().unwrap();
        let (read, warning) = UiState::from_json_lenient(&json);
        assert!(warning.is_none(), "{warning:?}");
        assert_eq!(read, state);
        assert_eq!(read.to_json().unwrap(), json);
    }
}
