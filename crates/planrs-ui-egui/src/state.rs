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
//!
//! J8 (ADR-0016, contrat L0) : préférences de la vue des tâches
//! ([`TaskViewPrefs`] : niveau affiché, surbrillance du critique), modèle
//! de mise en forme par défaut, modèles personnels ([`NamedStyle`]) et
//! écriture du style dans le fichier ; même règle, la version reste 1 (un
//! état J7 se lit sans avertissement). Filtre, tri et repli sont des vues
//! non persistées ([`TaskViewState`]).

use planrs_automation::filter::{FilterPreset, TaskFilter};
use planrs_render::style::{DocStyle, TemplateId};
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

/// Modèles personnels gardés au plus (J8).
pub const MAX_USER_TEMPLATES: usize = 32;

/// Longueur maximale du nom d'un modèle personnel, en caractères (J8).
pub const MAX_USER_TEMPLATE_NAME: usize = 64;

/// Niveau hiérarchique affiché au plus par « Afficher jusqu'au niveau n »
/// (H-UI-34).
pub const MAX_OUTLINE_LEVEL: u8 = 9;

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
    /// Préférences de la vue des tâches (J8).
    pub task_view: TaskViewPrefs,
    /// Modèle des documents sans style (J8, H-UI-43 : Classique jusqu'au
    /// lot L15).
    pub default_template: TemplateId,
    /// Modèles personnels, [`MAX_USER_TEMPLATES`] au plus (J8).
    pub user_templates: Vec<NamedStyle>,
    /// Écrire le style choisi dans le fichier (`<?planrs-style?>`, J8,
    /// H-R12) ; faux : jamais.
    pub write_style_to_file: bool,
}

/// Préférences persistées de la vue des tâches (J8, ADR-0016 D2) : seul
/// « Afficher jusqu'au niveau n » et la surbrillance du critique sont
/// gardés ; filtre, tri et repli ne le sont pas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskViewPrefs {
    /// Niveau affiché (1 à 9 ; `None` : tous).
    pub outline_level: Option<u8>,
    /// Surbrillance du chemin critique (H-UI-38, vrai par défaut).
    pub highlight_critical: bool,
}

impl Default for TaskViewPrefs {
    fn default() -> Self {
        Self {
            outline_level: None,
            highlight_critical: true,
        }
    }
}

/// Modèle personnel : nom et style (modèle de base et surcharges).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedStyle {
    /// Nom affiché (≤ [`MAX_USER_TEMPLATE_NAME`] caractères).
    pub name: String,
    /// Style.
    pub style: DocStyle,
}

/// Choix de filtre de la vue des tâches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilterChoice {
    /// Filtre prédéfini.
    Preset(FilterPreset),
    /// Filtre personnalisé.
    Custom(TaskFilter),
}

impl Default for FilterChoice {
    fn default() -> Self {
        Self::Preset(FilterPreset::All)
    }
}

/// Clé de tri (H-UI-31).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortKey {
    /// Ordre des ID (Maj+F3).
    #[default]
    Id,
    /// Nom.
    Name,
    /// Début.
    Start,
    /// Fin.
    Finish,
    /// Durée.
    Duration,
    /// % achevé.
    PercentComplete,
    /// Marge totale.
    TotalSlack,
}

impl SortKey {
    /// Clés proposées par la fenêtre « Trier » (J8, lot L9), dans l'ordre.
    pub const ALL: [Self; 7] = [
        Self::Id,
        Self::Name,
        Self::Start,
        Self::Finish,
        Self::Duration,
        Self::PercentComplete,
        Self::TotalSlack,
    ];

    /// Colonne dont le titre nomme la clé.
    #[must_use]
    pub const fn column(self) -> Column {
        match self {
            Self::Id => Column::Id,
            Self::Name => Column::Name,
            Self::Start => Column::Start,
            Self::Finish => Column::Finish,
            Self::Duration => Column::Duration,
            Self::PercentComplete => Column::PercentComplete,
            Self::TotalSlack => Column::TotalSlack,
        }
    }
}

/// Tri de la vue des tâches (sans renumérotation).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortSpec {
    /// Clé.
    pub key: SortKey,
    /// Décroissant.
    pub descending: bool,
    /// « Conserver la structure » (tri des frères, coché par défaut).
    pub keep_outline: bool,
}

/// État de la vue des tâches (J8, ADR-0016 D2) : une vue, rien d'écrit
/// (P50). Contrat posé par le lot L0, lu par `RowView` en L5.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskViewState {
    /// Filtre.
    pub filter: FilterChoice,
    /// Tri (`None` : ordre des ID).
    pub sort: Option<SortSpec>,
    /// Niveau affiché (`None` : tous).
    pub outline_level: Option<u8>,
    /// Surbrillance du chemin critique.
    pub highlight_critical: bool,
}

impl TaskViewState {
    /// Filtre effectif (préréglage déplié).
    #[must_use]
    pub fn task_filter(&self) -> TaskFilter {
        match &self.filter {
            FilterChoice::Preset(preset) => TaskFilter::from_preset(preset),
            FilterChoice::Custom(filter) => filter.clone(),
        }
    }

    /// Un filtre est posé (autre que « Toutes les tâches »).
    #[must_use]
    pub fn is_filtered(&self) -> bool {
        match &self.filter {
            FilterChoice::Preset(FilterPreset::All) => false,
            FilterChoice::Preset(_) => true,
            FilterChoice::Custom(filter) => !filter.is_empty(),
        }
    }

    /// Un tri est posé (autre que l'ordre des ID croissant).
    #[must_use]
    pub fn is_sorted(&self) -> bool {
        self.sort
            .is_some_and(|spec| spec.key != SortKey::Id || spec.descending)
    }

    /// Sous tri ou filtre, les opérations de structure qui déplacent des
    /// lignes sont grisées (H-UI-32, J8 lot L9).
    #[must_use]
    pub fn reorder_locked(&self) -> bool {
        self.is_filtered() || self.is_sorted()
    }
}

impl Default for TaskViewState {
    fn default() -> Self {
        let prefs = TaskViewPrefs::default();
        Self {
            filter: FilterChoice::default(),
            sort: None,
            outline_level: prefs.outline_level,
            highlight_critical: prefs.highlight_critical,
        }
    }
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
            task_view: TaskViewPrefs::default(),
            default_template: TemplateId::Classique,
            user_templates: Vec::new(),
            write_style_to_file: true,
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
            task_view: reader.field("task_view", defaults.task_view),
            default_template: reader.field("default_template", defaults.default_template),
            user_templates: reader.list("user_templates", defaults.user_templates),
            write_style_to_file: reader.field("write_style_to_file", defaults.write_style_to_file),
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
        self.user_templates.truncate(MAX_USER_TEMPLATES);
        for t in &mut self.user_templates {
            if t.name.chars().count() > MAX_USER_TEMPLATE_NAME {
                t.name = t.name.chars().take(MAX_USER_TEMPLATE_NAME).collect();
            }
        }
        self.task_view.outline_level = self
            .task_view
            .outline_level
            .map(|n| n.clamp(1, MAX_OUTLINE_LEVEL));
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
        self.user_templates.truncate(MAX_USER_TEMPLATES);
        for t in &mut self.user_templates {
            if t.name.chars().count() > MAX_USER_TEMPLATE_NAME {
                t.name = t.name.chars().take(MAX_USER_TEMPLATE_NAME).collect();
            }
        }
        self.task_view.outline_level = self
            .task_view
            .outline_level
            .map(|n| n.clamp(1, MAX_OUTLINE_LEVEL));
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

    /// Champ liste `name` (J8, modèles personnels) : chaque entrée est lue
    /// seule ; une entrée illisible est écartée et le champ relevé une
    /// fois. Pas un tableau → `default`, relevé.
    fn list<T: serde::de::DeserializeOwned>(&mut self, name: &'static str, default: Vec<T>) -> Vec<T> {
        let Some(raw) = self.object.get(name) else {
            return default;
        };
        let Some(entries) = raw.as_array() else {
            log::warn!("état d'interface : champ « {name} » ignoré : tableau attendu");
            self.bad.push(name);
            return default;
        };
        let mut out = Vec::new();
        let mut skipped = false;
        for entry in entries {
            match T::deserialize(entry) {
                Ok(v) => out.push(v),
                Err(error) => {
                    log::warn!("état d'interface : entrée de « {name} » ignorée : {error}");
                    skipped = true;
                }
            }
        }
        if skipped {
            self.bad.push(name);
        }
        out
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

/// Refus d'enregistrement d'un modèle personnel (J8, H-UI-61).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UserTemplateError {
    /// Nom vide ou de plus de [`MAX_USER_TEMPLATE_NAME`] caractères.
    #[error("nom de modèle de 1 à {MAX_USER_TEMPLATE_NAME} caractères attendu")]
    BadName,
    /// [`MAX_USER_TEMPLATES`] modèles déjà enregistrés (un nom existant
    /// reste remplaçable).
    #[error("{MAX_USER_TEMPLATES} modèles personnels au plus")]
    Full,
}

/// Style du document et modèles (J8, lot L6 ; ADR-0016 D7).
impl UiState {
    /// Style effectif d'un document (H-UI-43) : son style s'il en a un
    /// lisible, sinon le modèle par défaut des préférences.
    #[must_use]
    pub fn effective_style(&self, project: &planrs_core::Project) -> planrs_render::style::PlanStyle {
        planrs_automation::command::style::effective_style(project, self.default_template)
    }

    /// Options d'écriture MSPDI selon les préférences : style du document
    /// écrit (`<?planrs-style?>`) seulement si `write_style_to_file`.
    #[must_use]
    pub fn write_options(&self) -> planrs_mspdi::write::WriteOptions {
        planrs_mspdi::write::WriteOptions {
            style: self.write_style_to_file,
            ..planrs_mspdi::write::WriteOptions::default()
        }
    }

    /// Enregistre un modèle personnel (« Enregistrer comme modèle »,
    /// H-UI-61) : nom rogné, un modèle de même nom est remplacé sur place,
    /// sinon ajouté en fin de liste.
    ///
    /// # Errors
    ///
    /// [`UserTemplateError::BadName`], [`UserTemplateError::Full`].
    pub fn save_user_template(&mut self, name: &str, style: DocStyle) -> Result<(), UserTemplateError> {
        let name = name.trim();
        let n = name.chars().count();
        if n == 0 || n > MAX_USER_TEMPLATE_NAME {
            return Err(UserTemplateError::BadName);
        }
        if let Some(t) = self.user_templates.iter_mut().find(|t| t.name == name) {
            t.style = style;
            return Ok(());
        }
        if self.user_templates.len() >= MAX_USER_TEMPLATES {
            return Err(UserTemplateError::Full);
        }
        self.user_templates.push(NamedStyle {
            name: name.to_owned(),
            style,
        });
        Ok(())
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
    use planrs_automation::filter::FilterPreset;
    use planrs_render::style::TemplateId;

    use super::{
        FilterChoice, GanttStyle, LanguagePref, MAX_OUTLINE_LEVEL, MAX_RECENT_FILES, MAX_USER_TEMPLATE_NAME,
        MAX_USER_TEMPLATES, NamedStyle, RecentFile, StateLoadError, TABLE_WIDTH_DEFAULT, TABLE_WIDTH_MAX,
        TABLE_WIDTH_MIN, TaskViewPrefs, TaskViewState, ThemePref, UI_STATE_VERSION, UiState, ViewKind,
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
        // Colonnes J7 et colonne « Mode » (J8, lot L11) ajoutées masquées.
        let added: Vec<ColumnState> = state.columns.iter().copied().skip(8).collect();
        assert_eq!(added.len(), 6);
        assert!(added.iter().all(|c| !c.visible));
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

    /// État J7 (sans les champs J8) : lu sans avertissement, champs J8 à
    /// leur défaut (contrat J8 L0 : la version reste 1).
    #[test]
    fn ui_state_v1_j7_reads_without_new_fields() {
        const J7: &str = include_str!("../tests/fixtures/ui_state_v1_j7.json");
        assert!(!J7.contains("task_view") && !J7.contains("default_template"));
        let (state, warning) = UiState::from_json_lenient(J7);
        assert!(warning.is_none(), "{warning:?}");
        assert_eq!(state, UiState::default());
        assert_eq!(state.default_template, TemplateId::Classique);
        assert!(state.task_view.highlight_critical && state.write_style_to_file);
    }

    /// Champs J8 relus à l'identique ; bornes appliquées ; une entrée de
    /// modèle personnel illisible est écartée seule.
    #[test]
    fn ui_state_v1_roundtrip() {
        let sobre = planrs_render::style::parse_doc_style(r#"{"base":"sobre","v":1}"#).unwrap();
        let state = UiState {
            task_view: TaskViewPrefs {
                outline_level: Some(2),
                highlight_critical: false,
            },
            default_template: TemplateId::Sobre,
            user_templates: vec![NamedStyle {
                name: "Agence".to_owned(),
                style: sobre.clone(),
            }],
            write_style_to_file: false,
            ..UiState::default()
        };
        let (read, warning) = UiState::from_json_lenient(&state.to_json().unwrap());
        assert!(warning.is_none(), "{warning:?}");
        assert_eq!(read, state);

        let json = r#"{"task_view":{"outline_level":40,"highlight_critical":true},
            "user_templates":[{"name":"A","style":{"v":1,"base":"sobre"}},{"name":"B","style":{"v":1,"base":"inconnu"}}]}"#;
        let (read, warning) = UiState::from_json_lenient(json);
        assert_eq!(read.task_view.outline_level, Some(MAX_OUTLINE_LEVEL));
        assert_eq!(read.user_templates.len(), 1);
        assert_eq!(read.user_templates[0].style, sobre);
        assert!(matches!(warning, Some(StateLoadError::Fields(f)) if f == ["user_templates"]));
        let many: Vec<NamedStyle> = (0..40)
            .map(|i| NamedStyle {
                name: format!("{i}{}", "x".repeat(100)),
                style: sobre.clone(),
            })
            .collect();
        let state = UiState {
            user_templates: many,
            ..UiState::default()
        }
        .normalized();
        assert_eq!(state.user_templates.len(), MAX_USER_TEMPLATES);
        assert!(
            state
                .user_templates
                .iter()
                .all(|t| t.name.chars().count() == MAX_USER_TEMPLATE_NAME)
        );
        assert_eq!(
            TaskViewState::default().filter,
            FilterChoice::Preset(FilterPreset::All)
        );
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

    fn project() -> planrs_core::Project {
        let now: planrs_core::Moment = "2026-10-19T08:00".parse().unwrap();
        planrs_core::Project::new(planrs_core::ProjectInit {
            name: "P".to_owned(),
            anchor: planrs_core::ScheduleAnchor::FromStart(now),
            current_date: now,
            calendar_names: planrs_core::BuiltinCalendarNames::default(),
        })
    }

    /// H-UI-43 : document sans style ⇒ modèle par défaut des préférences
    /// (Classique jusqu'au lot L15) ; document stylé ⇒ son style.
    #[test]
    fn h_ui_43_default_template_applied() {
        use planrs_render::style::template;
        let mut state = UiState::default();
        let mut p = project();
        assert_eq!(state.effective_style(&p), template(TemplateId::Classique));
        state.default_template = TemplateId::Sobre;
        assert_eq!(state.effective_style(&p), template(TemplateId::Sobre));
        planrs_mspdi::style_pi::set(&mut p.extras, Some(r#"{"base":"chantier","v":1}"#.to_owned()));
        assert_eq!(state.effective_style(&p), template(TemplateId::Chantier));
        // Style illisible : modèle par défaut.
        planrs_mspdi::style_pi::set(&mut p.extras, Some("{".to_owned()));
        assert_eq!(state.effective_style(&p), template(TemplateId::Sobre));
    }

    #[test]
    fn write_options_follow_preference() {
        let mut state = UiState::default();
        assert!(state.write_options().style);
        state.write_style_to_file = false;
        assert!(!state.write_options().style);
    }

    #[test]
    fn save_user_template_bounds() {
        use super::UserTemplateError;
        use planrs_render::style::DocStyle;
        let doc = |base| DocStyle {
            v: 1,
            base,
            over: planrs_render::style::StylePatch::default(),
        };
        let mut state = UiState::default();
        state
            .save_user_template("  Client  ", doc(TemplateId::Sobre))
            .unwrap();
        state
            .save_user_template("Client", doc(TemplateId::Chantier))
            .unwrap();
        assert_eq!(state.user_templates.len(), 1);
        assert_eq!(state.user_templates[0].name, "Client");
        assert_eq!(state.user_templates[0].style.base, TemplateId::Chantier);
        assert_eq!(
            state.save_user_template(" ", doc(TemplateId::Sobre)),
            Err(UserTemplateError::BadName)
        );
        let long = "x".repeat(MAX_USER_TEMPLATE_NAME + 1);
        assert_eq!(
            state.save_user_template(&long, doc(TemplateId::Sobre)),
            Err(UserTemplateError::BadName)
        );
        for i in 1..MAX_USER_TEMPLATES {
            state
                .save_user_template(&format!("M{i}"), doc(TemplateId::Sobre))
                .unwrap();
        }
        assert_eq!(
            state.save_user_template("Un de plus", doc(TemplateId::Sobre)),
            Err(UserTemplateError::Full)
        );
        // Un nom existant reste remplaçable.
        state
            .save_user_template("M1", doc(TemplateId::Presentation))
            .unwrap();
        assert_eq!(state.user_templates.len(), MAX_USER_TEMPLATES);
    }
}
