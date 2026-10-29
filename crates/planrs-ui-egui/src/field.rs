//! Catalogue des champs de tâche proposés comme colonnes (seconde passe
//! UX, ADR-0019 D7, H-UI-114, H-UI-117).
//!
//! Contrat posé par le lot L0 : types, catégories ([`category`]),
//! [`catalog`] réduit aux 14 colonnes d'avant la passe et [`search`]
//! (sous-chaîne sans accents ni casse, titre et alias). Le lot L6 étend le
//! catalogue à tous les champs de [`Column::FIELDS`] (sélecteur, insertion),
//! L7 fixe leur disponibilité finale, L9 ajoute les champs personnalisés.
//! Aucune indexation pouvant paniquer (règle 10).

use std::cell::RefCell;
use std::collections::BTreeMap;

use planrs_core::model::custom::CustomFieldId;

use crate::column::{CellAlign, Column};
use crate::i18n::{Language, TextId, Texts};

/// Catégorie du sélecteur de champs (H-UI-117).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FieldCategory {
    /// Général.
    General,
    /// Planification.
    Scheduling,
    /// Avancement.
    Progress,
    /// Ligne de base.
    Baseline,
    /// Écarts.
    Variance,
    /// Marges.
    Slack,
    /// Ressources.
    Resources,
    /// Coûts (grisés : non calculés dans cette version).
    Costs,
    /// Champs personnalisés.
    Custom,
}

impl FieldCategory {
    /// Toutes les catégories, dans l'ordre du sélecteur.
    pub const ALL: [Self; 9] = [
        Self::General,
        Self::Scheduling,
        Self::Progress,
        Self::Baseline,
        Self::Variance,
        Self::Slack,
        Self::Resources,
        Self::Costs,
        Self::Custom,
    ];

    /// Titre traduit de la catégorie.
    #[must_use]
    pub const fn title(self) -> TextId {
        match self {
            Self::General => TextId::FieldCatGeneral,
            Self::Scheduling => TextId::FieldCatScheduling,
            Self::Progress => TextId::FieldCatProgress,
            Self::Baseline => TextId::FieldCatBaseline,
            Self::Variance => TextId::FieldCatVariance,
            Self::Slack => TextId::FieldCatSlack,
            Self::Resources => TextId::FieldCatResources,
            Self::Costs => TextId::FieldCatCosts,
            Self::Custom => TextId::FieldCatCustom,
        }
    }
}

/// Disponibilité d'un champ dans le sélecteur (H-UI-114, H-UI-117).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    /// Affichable et modifiable (le registre a la commande).
    Ready,
    /// Affichable, en lecture seule (valeur calculée ou sans commande).
    ReadOnly,
    /// Grisé, avec sa raison (coûts : non calculés dans cette version).
    Unavailable(TextId),
}

/// Raison des coûts grisés (H-UI-117).
pub const COSTS_REASON: TextId = TextId::FieldCostsNotComputed;

/// Entrée du catalogue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldDesc {
    /// Colonne du champ.
    pub column: Column,
    /// Titre traduit.
    pub title: TextId,
    /// Catégorie.
    pub category: FieldCategory,
    /// Alignement des cellules.
    pub align: CellAlign,
    /// Disponibilité.
    pub availability: Availability,
}

/// Entrée d'une colonne : titre, catégorie et alignement de [`Column`].
const fn desc(column: Column, availability: Availability) -> FieldDesc {
    FieldDesc {
        column,
        title: column.title(),
        category: category(column),
        align: column.align(),
        availability,
    }
}

/// Disponibilité d'un champ au lot L6 (H-UI-117, liste) : les 14 colonnes
/// d'avant la passe gardent la leur (modifiables là où la table l'était
/// déjà) ; les coûts sont grisés ([`COSTS_REASON`]) ; les autres champs
/// s'affichent en lecture seule, vides, jusqu'au lot L7 qui sert leurs
/// valeurs et fixe leur disponibilité finale.
#[must_use]
pub const fn availability(column: Column) -> Availability {
    match column {
        Column::Name
        | Column::Duration
        | Column::Start
        | Column::Finish
        | Column::Predecessors
        | Column::PercentComplete => Availability::Ready,
        Column::Cost
        | Column::FixedCost
        | Column::ActualCost
        | Column::RemainingCost
        | Column::BaselineCost
        | Column::CostVariance => Availability::Unavailable(COSTS_REASON),
        _ => Availability::ReadOnly,
    }
}

/// Catalogue complet des champs de tâche (H-UI-117) : les 14 colonnes
/// d'avant la passe ([`Column::ALL`]) puis les champs de
/// [`Column::FIELDS`], dans l'ordre de l'énumération ; les champs
/// personnalisés sont ajoutés par le lot L9.
static CATALOG: std::sync::LazyLock<Vec<FieldDesc>> = std::sync::LazyLock::new(|| {
    Column::ALL
        .into_iter()
        .chain(Column::FIELDS)
        .map(|column| desc(column, availability(column)))
        .collect()
});

/// Champs proposés par le sélecteur, dans l'ordre du catalogue.
#[must_use]
pub fn catalog() -> &'static [FieldDesc] {
    CATALOG.as_slice()
}

/// Entrée du catalogue d'une colonne (`None` : champ hors catalogue, comme
/// un champ personnalisé avant le lot L9).
#[must_use]
pub fn describe(column: Column) -> Option<&'static FieldDesc> {
    catalog().iter().find(|d| d.column == column)
}

/// Catégorie d'une colonne (H-UI-117).
#[must_use]
pub const fn category(column: Column) -> FieldCategory {
    match column {
        Column::Id
        | Column::Name
        | Column::UniqueId
        | Column::Wbs
        | Column::OutlineLevel
        | Column::Summary
        | Column::Milestone
        | Column::Notes
        | Column::Created
        | Column::Priority
        | Column::Active => FieldCategory::General,
        Column::Duration
        | Column::Start
        | Column::Finish
        | Column::Predecessors
        | Column::Critical
        | Column::Mode
        | Column::Successors
        | Column::ConstraintType
        | Column::ConstraintDate
        | Column::Deadline
        | Column::TaskType
        | Column::EffortDriven
        | Column::Work
        | Column::EarlyStart
        | Column::EarlyFinish
        | Column::LateStart
        | Column::LateFinish => FieldCategory::Scheduling,
        Column::TotalSlack | Column::FreeSlack => FieldCategory::Slack,
        Column::PercentComplete
        | Column::PercentWorkComplete
        | Column::ActualStart
        | Column::ActualFinish
        | Column::ActualDuration
        | Column::RemainingDuration
        | Column::ActualWork
        | Column::RemainingWork => FieldCategory::Progress,
        Column::ResourceNames => FieldCategory::Resources,
        Column::BaselineStart
        | Column::BaselineFinish
        | Column::BaselineDuration
        | Column::BaselineWork
        | Column::Baseline1Start
        | Column::Baseline1Finish
        | Column::Baseline1Duration
        | Column::Baseline1Work
        | Column::Baseline2Start
        | Column::Baseline2Finish
        | Column::Baseline2Duration
        | Column::Baseline2Work
        | Column::Baseline3Start
        | Column::Baseline3Finish
        | Column::Baseline3Duration
        | Column::Baseline3Work
        | Column::Baseline4Start
        | Column::Baseline4Finish
        | Column::Baseline4Duration
        | Column::Baseline4Work
        | Column::Baseline5Start
        | Column::Baseline5Finish
        | Column::Baseline5Duration
        | Column::Baseline5Work
        | Column::Baseline6Start
        | Column::Baseline6Finish
        | Column::Baseline6Duration
        | Column::Baseline6Work
        | Column::Baseline7Start
        | Column::Baseline7Finish
        | Column::Baseline7Duration
        | Column::Baseline7Work
        | Column::Baseline8Start
        | Column::Baseline8Finish
        | Column::Baseline8Duration
        | Column::Baseline8Work
        | Column::Baseline9Start
        | Column::Baseline9Finish
        | Column::Baseline9Duration
        | Column::Baseline9Work
        | Column::Baseline10Start
        | Column::Baseline10Finish
        | Column::Baseline10Duration
        | Column::Baseline10Work => FieldCategory::Baseline,
        Column::FinishVariance | Column::StartVariance | Column::DurationVariance | Column::WorkVariance => {
            FieldCategory::Variance
        }
        Column::Cost
        | Column::FixedCost
        | Column::ActualCost
        | Column::RemainingCost
        | Column::BaselineCost
        | Column::CostVariance => FieldCategory::Costs,
        Column::Custom(_) => FieldCategory::Custom,
    }
}

/// Texte replié pour la recherche : minuscules, sans accents latins
/// courants (« Durée » ⇒ « duree », « Œuvre » ⇒ « oeuvre »).
#[must_use]
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars().flat_map(char::to_lowercase) {
        match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => out.push('a'),
            'é' | 'è' | 'ê' | 'ë' => out.push('e'),
            'î' | 'ï' | 'í' | 'ì' => out.push('i'),
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => out.push('o'),
            'ù' | 'û' | 'ü' | 'ú' => out.push('u'),
            'ÿ' | 'ý' => out.push('y'),
            'ç' => out.push('c'),
            'ñ' => out.push('n'),
            'œ' => out.push_str("oe"),
            'æ' => out.push_str("ae"),
            other => out.push(other),
        }
    }
    out
}

thread_local! {
    /// Textes de la dernière langue cherchée : une recherche par frappe ne
    /// relit pas les fichiers fluent.
    static TEXTS: RefCell<Option<Texts>> = const { RefCell::new(None) };
}

/// Champs dont le titre (langue `lang`) ou l'alias contient `query`
/// (sous-chaîne, sans accents ni casse ; vide : tout le catalogue), dans
/// l'ordre du catalogue, puis les champs personnalisés d'alias `aliases`
/// qui correspondent, dans l'ordre de leurs identifiants.
#[must_use]
pub fn search(query: &str, lang: Language, aliases: &BTreeMap<CustomFieldId, String>) -> Vec<Column> {
    let needle = fold(query.trim());
    let found = TEXTS.with(|cell| {
        let mut cache = cell.borrow_mut();
        let texts = match cache.take() {
            Some(texts) if texts.language() == lang => texts,
            _ => Texts::new(lang),
        };
        let found: Vec<Column> = catalog()
            .iter()
            .filter(|d| fold(&texts.get(d.title)).contains(&needle))
            .map(|d| d.column)
            .collect();
        *cache = Some(texts);
        found
    });
    let custom = aliases
        .iter()
        .filter(|(_, alias)| fold(alias).contains(&needle))
        .map(|(id, _)| Column::Custom(*id));
    let mut out = found;
    for column in custom {
        if !out.contains(&column) {
            out.push(column);
        }
    }
    out
}

/// Fenêtre de colonne ouverte (H-UI-114, H-UI-115, lot L6) : sélecteur de
/// champ (insertion au rang `at`) ou, si `settings` est posé, « Paramètres
/// de champ » de la colonne de rang `at`. Une seule à la fois, hébergée par
/// `ViewTransient::picker` (contrat L0) : la fenêtre « Paramètres de champ »
/// n'a pas de `DialogKind` (préférence d'affichage, aucun brouillon du
/// document), elle partage donc l'emplacement du sélecteur.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FieldPicker {
    /// Rang d'insertion dans `UiState.columns` (sélecteur) ou rang de la
    /// colonne réglée (`settings`).
    pub at: usize,
    /// Texte de recherche.
    pub query: String,
    /// Rang de l'entrée en surbrillance parmi les entrées choisissables
    /// de la liste filtrée ([`choices`]).
    pub highlighted: usize,
    /// Focus à donner à la zone de saisie à la prochaine image (ouverture).
    pub focus_pending: bool,
    /// « Paramètres de champ » ouverts (H-UI-115) à la place du sélecteur.
    pub settings: Option<Box<crate::dialog::field_settings::FieldSettingsDraft>>,
}

impl FieldPicker {
    /// Sélecteur ouvert pour une insertion au rang `at`, recherche vide,
    /// première entrée en surbrillance, focus à la recherche.
    #[must_use]
    pub fn insert_at(at: usize) -> Self {
        Self {
            at,
            focus_pending: true,
            ..Self::default()
        }
    }
}

/// Entrées choisissables du sélecteur pour la recherche `query` : champs
/// trouvés ([`search`]) qui ne sont pas grisés, dans l'ordre d'affichage
/// (par catégorie, puis ordre du catalogue). C'est la liste que ↑/↓
/// parcourent et qu'Entrée insère (H-UI-114).
#[must_use]
pub fn choices(query: &str, lang: Language, aliases: &BTreeMap<CustomFieldId, String>) -> Vec<Column> {
    let found = search(query, lang, aliases);
    let mut out = Vec::with_capacity(found.len());
    for cat in FieldCategory::ALL {
        out.extend(found.iter().copied().filter(|c| {
            category(*c) == cat
                && !matches!(
                    describe(*c).map(|d| d.availability),
                    Some(Availability::Unavailable(_))
                )
        }));
    }
    out
}

/// Textes posés par le lot L0 pour l'insertion de colonnes, servis par le
/// lot L6 (menu d'en-tête, sélecteur, « Ajouter une nouvelle colonne »,
/// « Paramètres de champ »).
pub const L6_TEXTS: [TextId; 16] = [
    TextId::MenuInsertColumn,
    TextId::MenuHideColumn,
    TextId::MenuFieldSettings,
    TextId::AddNewColumn,
    TextId::FieldPickerSearch,
    TextId::FieldPickerEmpty,
    TextId::FieldPickerShown,
    TextId::FieldSettingsTitle,
    TextId::FieldSettingsField,
    TextId::FieldSettingsColumnTitle,
    TextId::FieldSettingsAlignTitle,
    TextId::FieldSettingsAlignData,
    TextId::FieldSettingsWidth,
    TextId::AlignLeft,
    TextId::AlignCenter,
    TextId::AlignRight,
];

/// Textes posés par le lot L0 pour les colonnes, servis par L6 : refus
/// (64 colonnes, déplacer l'ID), sous-menu « Affichage du tableau ».
pub const L6_MORE_TEXTS: [TextId; 3] = [
    TextId::ColumnsMaxReached,
    TextId::ColumnIdStaysFirst,
    TextId::MenuTableDisplay,
];

/// Textes posés par le lot L0 pour les champs personnalisés, servis par
/// le lot L9 (fenêtre Projet → Champs personnalisés…, types, titre avec
/// alias, libellé du lot d'annulation).
pub const L9_TEXTS: [TextId; 10] = [
    TextId::MenuCustomFields,
    TextId::CustomFieldsTitle,
    TextId::CustomFieldsRename,
    TextId::CustomFieldsAliasTitle,
    TextId::CustomFieldsBatchLabel,
    TextId::CustomFieldsAliasTooLong,
    TextId::CustomTypeText,
    TextId::CustomTypeNumber,
    TextId::CustomTypeDate,
    TextId::CustomTypeFlag,
];

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "tests : un échec doit s'arrêter net et lisiblement"
)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use planrs_core::model::custom::{CustomFieldId, CustomKind};

    use super::{FieldCategory, catalog, category, fold, search};
    use crate::column::Column;
    use crate::i18n::{Language, Texts};

    /// H-UI-117 : chaque colonne au plus une fois ; chaque titre (de tous
    /// les champs, catégories et textes posés) traduit en français et en
    /// anglais ; le catalogue (lot L6) couvre les 14 colonnes et les champs
    /// de [`Column::FIELDS`].
    #[test]
    fn field_catalog_unique() {
        let columns: BTreeSet<Column> = catalog().iter().map(|d| d.column).collect();
        assert_eq!(columns.len(), catalog().len());
        assert_eq!(columns, Column::ALL.into_iter().chain(Column::FIELDS).collect());
        let mut ids: Vec<crate::i18n::TextId> = Column::ALL
            .into_iter()
            .chain(Column::FIELDS)
            .map(Column::title)
            .chain(FieldCategory::ALL.map(FieldCategory::title))
            .collect();
        ids.extend(super::L6_TEXTS);
        ids.extend(super::L6_MORE_TEXTS);
        ids.extend(super::L9_TEXTS);
        ids.push(super::COSTS_REASON);
        for lang in Language::ALL {
            let texts = Texts::new(lang);
            for id in &ids {
                let text = texts.get(*id);
                assert!(!text.is_empty() && text != id.key(), "{lang:?} {id:?}");
            }
        }
        // Titres distincts (une langue suffit à le montrer).
        let fr = Texts::new(Language::Fr);
        let titles: BTreeSet<String> = Column::ALL
            .into_iter()
            .chain(Column::FIELDS)
            .map(|c| fr.get(c.title()))
            .collect();
        assert_eq!(titles.len(), Column::COUNT + Column::FIELDS.len());
        assert_eq!(category(Column::FreeSlack), FieldCategory::Slack);
        assert_eq!(category(Column::Cost), FieldCategory::Costs);
        let custom = Column::Custom(CustomFieldId::new(CustomKind::Flag, 3).unwrap());
        assert_eq!(category(custom), FieldCategory::Custom);
        assert_eq!(custom.label(&fr), "Indicateur3");
        assert_eq!(custom.label(&Texts::new(Language::En)), "Flag3");
    }

    /// H-UI-114 : « duree » trouve « Durée », sans casse ; recherche vide
    /// ⇒ tout ; alias de champ personnalisé trouvé.
    #[test]
    fn field_search_accents() {
        let none = BTreeMap::new();
        assert_eq!(fold("Durée ŒUVRE"), "duree oeuvre");
        let first = |q: &str, lang| search(q, lang, &none).first().copied();
        assert_eq!(first("duree", Language::Fr), Some(Column::Duration));
        assert_eq!(first("DURÉE", Language::Fr), Some(Column::Duration));
        assert_eq!(first("duration", Language::En), Some(Column::Duration));
        assert!(search("duree", Language::Fr, &none).contains(&Column::RemainingDuration));
        assert_eq!(search("", Language::Fr, &none).len(), catalog().len());
        let found = search("fin de référence", Language::Fr, &none);
        assert_eq!(found.first(), Some(&Column::BaselineFinish));
        assert!(found.contains(&Column::Baseline10Finish));
        assert_eq!(search("zzz", Language::Fr, &none).len(), 0);
        let lot = CustomFieldId::new(CustomKind::Text, 1).unwrap();
        let aliases = BTreeMap::from([(lot, "Lot à bâtir".to_owned())]);
        assert_eq!(search("batir", Language::Fr, &aliases), [Column::Custom(lot)]);
    }
}
