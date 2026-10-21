//! État d'interface sérialisable (`UiState`), séparé du modèle.
//!
//! `UiState` regroupe les préférences et la position de lecture, en JSON
//! (`serde_json`), sans aucune donnée du projet ni aucun flottant : le JSON est
//! déterministe et `UiState: Eq`. Lecture tolérante, champ par champ : champ
//! absent → valeur par défaut, champ inconnu ignoré (une version plus récente
//! a pu écrire davantage), champ illisible → sa valeur par défaut sans perdre
//! les autres, entrée de colonne illisible (colonne d'une version future)
//! ignorée.
//!
//! J7 (ADR-0015 D6, contrat L0) : vue principale ([`ViewKind`]), style du
//! Gantt ([`GanttStyle`]) et fichiers récents ([`RecentFile`]), champs
//! ajoutés avec valeur par défaut : la version reste 1 (un état J6 se lit
//! sans avertissement).

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

/// Fichiers récents gardés au plus (H-UI-28).
pub const MAX_RECENT_FILES: usize = 10;

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
    /// Vue principale (J7).
    pub view: ViewKind,
    /// Style du Gantt (J7).
    pub gantt_style: GanttStyle,
    /// Fichiers récents, le plus récent en tête, [`MAX_RECENT_FILES`] au
    /// plus (J7, H-UI-28).
    pub recent: Vec<RecentFile>,
}

/// Vue principale (J7, ADR-0015 D6).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewKind {
    /// Diagramme de Gantt (table des tâches et barres).
    #[default]
    Gantt,
    /// Feuille des ressources (lot L10).
    ResourceSheet,
    /// Utilisation des ressources (lot L11).
    ResourceUsage,
}

/// Style du Gantt (J7, H-UI-22).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GanttStyle {
    /// Barres planifiées seules.
    #[default]
    Standard,
    /// Gantt de suivi : ligne de base 0, bande d'avancement (lot L7).
    Tracking,
}

/// Fichier récent (J7, H-UI-28) : chemin et nom affiché.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecentFile {
    /// Chemin du fichier.
    pub path: String,
    /// Nom affiché.
    pub name: String,
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
///
/// Le texte (`Display`) est inséré tel quel dans le message traduit
/// `state-load-warning` : il ne contient donc aucun mot d'une langue, seulement
/// des repères techniques (« JSON 1:5 », « v2 > v1 », noms de champs).
#[derive(Debug, thiserror::Error)]
pub enum StateLoadError {
    /// JSON illisible : l'état par défaut est utilisé. Texte : position de
    /// l'erreur (« JSON 1:5 »).
    #[error("JSON {}:{}", .0.line(), .0.column())]
    Json(#[from] serde_json::Error),
    /// JSON valide mais pas un objet : l'état par défaut est utilisé.
    #[error("JSON")]
    NotObject,
    /// État écrit par une version plus récente : état par défaut (le sens des
    /// champs a pu changer).
    #[error("v{found} > v{supported}")]
    TooNew {
        /// Version trouvée.
        found: u32,
        /// Version la plus récente comprise.
        supported: u32,
    },
    /// Champs illisibles remplacés par leur valeur par défaut, les autres
    /// étant conservés. Texte : noms JSON des champs (« theme, columns »).
    #[error("{}", .0.join(", "))]
    Fields(Vec<&'static str>),
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
            view: ViewKind::default(),
            gantt_style: GanttStyle::default(),
            recent: Vec::new(),
        }
    }
}

impl UiState {
    /// Lecture tolérante : jamais d'échec. Rend l'état normalisé et, s'il y a
    /// lieu, l'avertissement à afficher une fois.
    ///
    /// - JSON illisible : état par défaut et [`StateLoadError::Json`] ;
    ///   JSON valide qui n'est pas un objet : [`StateLoadError::NotObject`] ;
    /// - `version` supérieure à [`UI_STATE_VERSION`] : état par défaut et
    ///   [`StateLoadError::TooNew`] (le sens des champs a pu changer) ;
    /// - sinon, chaque champ est lu indépendamment : absent → défaut, inconnu
    ///   → ignoré, illisible → défaut et nom du champ dans
    ///   [`StateLoadError::Fields`] ; dans `columns`, une entrée illisible
    ///   est écartée (et signalée), les autres sont gardées.
    #[must_use]
    pub fn from_json_lenient(json: &str) -> (Self, Option<StateLoadError>) {
        let value = match serde_json::from_str::<serde_json::Value>(json) {
            Ok(value) => value,
            Err(error) => return (Self::default(), Some(StateLoadError::Json(error))),
        };
        let Some(object) = value.as_object() else {
            return (Self::default(), Some(StateLoadError::NotObject));
        };
        // La version est lue avant le reste : un état plus récent peut avoir
        // changé le sens d'un champ connu.
        if let Some(found) = object.get("version").and_then(serde_json::Value::as_u64)
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
        let mut reader = FieldReader {
            object,
            bad: Vec::new(),
        };
        let defaults = Self::default();
        let state = Self {
            version: reader.field("version", defaults.version),
            language: reader.field("language", defaults.language),
            theme: reader.field("theme", defaults.theme),
            zoom: reader.field("zoom", defaults.zoom),
            columns: reader.columns(defaults.columns),
            table_width: reader.field("table_width", defaults.table_width),
            diagnostics_open: reader.field("diagnostics_open", defaults.diagnostics_open),
            view: reader.field("view", defaults.view),
            gantt_style: reader.field("gantt_style", defaults.gantt_style),
            recent: reader.field("recent", defaults.recent),
        };
        let warning = (!reader.bad.is_empty()).then_some(StateLoadError::Fields(reader.bad));
        (state.normalized(), warning)
    }

    /// JSON compact déterministe (ordre des champs fixe, aucune map).
    ///
    /// # Errors
    ///
    /// Erreur de sérialisation de `serde_json` (non attendue pour ce type).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Inscrit `path` en tête des fichiers récents (H-UI-28) : une entrée
    /// déjà présente remonte (nom mis à jour), le plus ancien sort au-delà
    /// de [`MAX_RECENT_FILES`].
    pub fn remember_recent(&mut self, path: &str, name: &str) {
        self.recent.retain(|recent| recent.path != path);
        self.recent.insert(
            0,
            RecentFile {
                path: path.to_owned(),
                name: name.to_owned(),
            },
        );
        self.recent.truncate(MAX_RECENT_FILES);
    }

    /// Retire `path` des fichiers récents (fichier introuvable, H-UI-28).
    pub fn forget_recent(&mut self, path: &str) {
        self.recent.retain(|recent| recent.path != path);
    }

    /// Bornes appliquées, colonnes complétées et dédoublonnées dans l'ordre
    /// de [`Column::ALL`] (la première occurrence d'une colonne l'emporte),
    /// largeurs bornées à `[Column::MIN_WIDTH, Column::MAX_WIDTH]`, largeur
    /// de table bornée à `[TABLE_WIDTH_MIN, TABLE_WIDTH_MAX]`, ID et Nom
    /// forcés visibles, version ramenée à [`UI_STATE_VERSION`], fichiers
    /// récents coupés à [`MAX_RECENT_FILES`].
    #[must_use]
    pub fn normalized(mut self) -> Self {
        self.version = UI_STATE_VERSION;
        self.recent.truncate(MAX_RECENT_FILES);
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

    /// Affiche ou masque `column` (J7, lot L6) ; l'ID et le Nom restent
    /// visibles (H-UI-12). Rend vrai si la visibilité a changé.
    pub fn toggle_column(&mut self, column: Column) -> bool {
        if !column.hideable() {
            return false;
        }
        match self.columns.iter_mut().find(|state| state.column == column) {
            Some(state) => {
                state.visible = !state.visible;
                true
            }
            None => false,
        }
    }

    /// Largeur de `column`, bornée à `[Column::MIN_WIDTH, Column::MAX_WIDTH]`
    /// comme par [`UiState::normalized`] (J7, lot L6). Rend vrai si elle a
    /// changé.
    pub fn set_column_width(&mut self, column: Column, width: u16) -> bool {
        let width = width.clamp(Column::MIN_WIDTH, Column::MAX_WIDTH);
        match self.columns.iter_mut().find(|state| state.column == column) {
            Some(state) if state.width != width => {
                state.width = width;
                true
            }
            _ => false,
        }
    }
}

/// Lecteur champ par champ d'un objet JSON d'état : chaque champ illisible
/// prend sa valeur par défaut et son nom est relevé.
struct FieldReader<'a> {
    /// Objet JSON lu.
    object: &'a serde_json::Map<String, serde_json::Value>,
    /// Noms des champs illisibles, dans l'ordre de lecture.
    bad: Vec<&'static str>,
}

impl FieldReader<'_> {
    /// Champ `name` : absent → `default` ; illisible → `default`, relevé.
    fn field<T: serde::de::DeserializeOwned>(&mut self, name: &'static str, default: T) -> T {
        match self.object.get(name) {
            None => default,
            Some(raw) => T::deserialize(raw).unwrap_or_else(|error| {
                log::warn!("état d'interface : champ « {name} » ignoré : {error}");
                self.bad.push(name);
                default
            }),
        }
    }

    /// Champ `columns` : un tableau dont chaque entrée est lue seule ; une
    /// entrée illisible (colonne inconnue d'une version future, largeur hors
    /// type…) est écartée et le champ relevé une fois. Pas un tableau →
    /// `default`, relevé.
    fn columns(&mut self, default: Vec<ColumnState>) -> Vec<ColumnState> {
        const NAME: &str = "columns";
        let Some(raw) = self.object.get(NAME) else {
            return default;
        };
        let Some(entries) = raw.as_array() else {
            log::warn!("état d'interface : champ « {NAME} » ignoré : tableau attendu");
            self.bad.push(NAME);
            return default;
        };
        let mut columns = Vec::with_capacity(entries.len().min(Column::COUNT));
        let mut skipped = false;
        for entry in entries {
            match ColumnState::deserialize(entry) {
                Ok(state) => columns.push(state),
                Err(error) => {
                    log::warn!("état d'interface : colonne ignorée : {error}");
                    skipped = true;
                }
            }
        }
        if skipped {
            self.bad.push(NAME);
        }
        columns
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
        GanttStyle, LanguagePref, MAX_RECENT_FILES, RecentFile, StateLoadError, TABLE_WIDTH_DEFAULT,
        TABLE_WIDTH_MAX, TABLE_WIDTH_MIN, ThemePref, UI_STATE_VERSION, UiState, ViewKind,
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

    /// État J6 (sans `view`, `gantt_style` ni `recent`, 8 colonnes) : lu
    /// sans avertissement, champs J7 à leur défaut, colonnes J7 ajoutées
    /// masquées (contrat J7 L0 : la version reste 1).
    #[test]
    fn ui_state_v1_without_new_fields_reads() {
        const J6: &str = include_str!("../tests/fixtures/ui_state_v1_j6.json");
        assert!(!J6.contains("gantt_style") && !J6.contains("percent_complete"));
        let (state, warning) = UiState::from_json_lenient(J6);
        assert!(warning.is_none(), "{warning:?}");
        assert_eq!(state, UiState::default());
        assert_eq!(state.view, ViewKind::Gantt);
        assert_eq!(state.gantt_style, GanttStyle::Standard);
        assert_eq!(state.recent.len(), 0);
        let j7: Vec<ColumnState> = state.columns.iter().copied().skip(8).collect();
        assert_eq!(j7.len(), 5);
        assert!(j7.iter().all(|c| !c.visible));
    }

    /// Champs J7 relus ; fichiers récents coupés à 10 (H-UI-28).
    #[test]
    fn ui_state_j7_fields_round_trip() {
        let recent: Vec<RecentFile> = (0..12)
            .map(|i| RecentFile {
                path: format!("/p/{i}.xml"),
                name: format!("{i}.xml"),
            })
            .collect();
        let state = UiState {
            view: ViewKind::ResourceUsage,
            gantt_style: GanttStyle::Tracking,
            recent,
            ..UiState::default()
        };
        let (read, warning) = UiState::from_json_lenient(&state.to_json().unwrap());
        assert!(warning.is_none(), "{warning:?}");
        assert_eq!(read.view, ViewKind::ResourceUsage);
        assert_eq!(read.gantt_style, GanttStyle::Tracking);
        assert_eq!(read.recent.len(), MAX_RECENT_FILES);
        assert_eq!(read.recent[0].name, "0.xml");
        let (read, warning) = UiState::from_json_lenient(r#"{"view":"calendar"}"#);
        assert_eq!(read.view, ViewKind::Gantt);
        assert!(matches!(warning, Some(StateLoadError::Fields(f)) if f == ["view"]));
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
        for garbage in ["", "{", "not json"] {
            let (state, warning) = UiState::from_json_lenient(garbage);
            assert_eq!(state, UiState::default(), "{garbage:?}");
            assert!(
                matches!(warning, Some(StateLoadError::Json(_))),
                "{garbage:?} : {warning:?}"
            );
        }
        for not_object in ["null", "42", "[]", "\"x\""] {
            let (state, warning) = UiState::from_json_lenient(not_object);
            assert_eq!(state, UiState::default(), "{not_object:?}");
            assert!(
                matches!(warning, Some(StateLoadError::NotObject)),
                "{not_object:?} : {warning:?}"
            );
        }
        let (state, warning) = UiState::from_json_lenient("{\"table_width\": -5}");
        assert_eq!(state, UiState::default());
        assert!(
            matches!(&warning, Some(StateLoadError::Fields(fields)) if fields == &["table_width"]),
            "{warning:?}"
        );
    }

    /// D-fix-11 : un champ illisible prend sa valeur par défaut, les autres
    /// sont conservés ; une colonne inconnue (version future) est écartée
    /// sans perdre les autres colonnes.
    #[test]
    fn ui_state_lenient_field_by_field() {
        // Langue inconnue : seul ce champ revient au défaut, le thème reste.
        let (state, warning) = UiState::from_json_lenient(r#"{"language":"de","theme":"dark"}"#);
        assert_eq!(state.theme, ThemePref::Dark);
        assert_eq!(state.language, LanguagePref::System);
        assert!(
            matches!(&warning, Some(StateLoadError::Fields(f)) if f == &["language"]),
            "{warning:?}"
        );
        // Plusieurs champs illisibles, d'autres valides.
        let json = r#"{"version":"9","language":"en","theme":42,"zoom":"century",
            "table_width":"wide","diagnostics_open":true}"#;
        let (state, warning) = UiState::from_json_lenient(json);
        assert_eq!(state.language, LanguagePref::En);
        assert_eq!(state.theme, ThemePref::System);
        assert_eq!(state.zoom, TimeScaleZoom::Week);
        assert_eq!(state.table_width, TABLE_WIDTH_DEFAULT);
        assert!(state.diagnostics_open);
        assert_eq!(state.version, UI_STATE_VERSION);
        match &warning {
            Some(StateLoadError::Fields(fields)) => {
                assert_eq!(fields, &["version", "theme", "zoom", "table_width"]);
            }
            other => panic!("attendu Fields, obtenu {other:?}"),
        }
        // Texte neutre (inséré dans le message traduit `state-load-warning`).
        assert_eq!(
            warning.map(|w| w.to_string()).as_deref(),
            Some("version, theme, zoom, table_width")
        );
        // Colonne d'une version future : écartée, les autres colonnes et le
        // thème sont gardés.
        let json = r#"{"theme":"light","columns":[
            {"column":"future_col","visible":true,"width":50},
            {"column":"duration","visible":false,"width":70},
            {"column":"total_slack","visible":true,"width":"x"}]}"#;
        let (state, warning) = UiState::from_json_lenient(json);
        assert_eq!(state.theme, ThemePref::Light);
        assert_eq!(
            state.column(Column::Duration),
            Some(&ColumnState {
                column: Column::Duration,
                visible: false,
                width: 70
            })
        );
        assert_eq!(
            state.column(Column::TotalSlack),
            Some(&ColumnState::default_for(Column::TotalSlack))
        );
        assert_eq!(state.columns.len(), Column::COUNT);
        assert!(
            matches!(&warning, Some(StateLoadError::Fields(f)) if f == &["columns"]),
            "{warning:?}"
        );
        // `columns` qui n'est pas un tableau : colonnes par défaut.
        let (state, warning) = UiState::from_json_lenient(r#"{"columns":{"id":1},"zoom":"day"}"#);
        assert_eq!(state.columns, UiState::default().columns);
        assert_eq!(state.zoom, TimeScaleZoom::Day);
        assert!(matches!(warning, Some(StateLoadError::Fields(_))));
    }

    /// Les textes d'avertissement ne contiennent aucun mot d'une langue
    /// (insérés tels quels dans un message traduit).
    #[test]
    fn ui_state_warning_text_is_language_neutral() {
        let (_, json) = UiState::from_json_lenient("{");
        assert!(json.unwrap().to_string().starts_with("JSON "));
        let (_, too_new) = UiState::from_json_lenient(r#"{"version":2}"#);
        assert_eq!(too_new.unwrap().to_string(), "v2 > v1");
        assert_eq!(StateLoadError::NotObject.to_string(), "JSON");
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
