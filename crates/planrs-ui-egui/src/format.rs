//! Formateurs d'affichage purs (durée, prédécesseurs, date, marge), sans egui
//! ni fluent.
//!
//! Les libellés sont résolus une fois par langue depuis les `.ftl`
//! (`Texts::format_labels`) : les formateurs ne voient que des chaînes, ce qui
//! les rend testables sans fluent. Les nombres sont écrits en arithmétique
//! entière à partir du point fixe du cœur (jamais de flottant). Le format
//! MS Project d'une durée ou d'un prédécesseur n'existe pas encore dans le
//! cœur : API à migrer en J4 (`docs/ui-needs.md`), hypothèses H-UI.

use planrs_core::{ConversionFactors, Duration, DurationUnit, Link, Minutes, Moment, Project, TimeBasis};

/// Libellé d'une unité au singulier et au pluriel (« jour » / « jours »).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnitLabel {
    /// Forme employée quand la valeur vaut exactement 1.
    pub one: String,
    /// Forme employée sinon.
    pub other: String,
}

/// Libellés de toutes les unités de durée, ouvrées et écoulées (champs
/// nommés : aucune indexation).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnitLabels {
    /// Minute ouvrée.
    pub minute: UnitLabel,
    /// Heure ouvrée.
    pub hour: UnitLabel,
    /// Jour ouvré.
    pub day: UnitLabel,
    /// Semaine ouvrée.
    pub week: UnitLabel,
    /// Mois ouvré.
    pub month: UnitLabel,
    /// Minute écoulée.
    pub elapsed_minute: UnitLabel,
    /// Heure écoulée.
    pub elapsed_hour: UnitLabel,
    /// Jour écoulé.
    pub elapsed_day: UnitLabel,
    /// Semaine écoulée.
    pub elapsed_week: UnitLabel,
    /// Mois écoulé.
    pub elapsed_month: UnitLabel,
}

impl UnitLabels {
    /// Libellé d'une unité dans une base de temps.
    #[must_use]
    pub const fn get(&self, unit: DurationUnit, basis: TimeBasis) -> &UnitLabel {
        match (basis, unit) {
            (TimeBasis::Working, DurationUnit::Minute) => &self.minute,
            (TimeBasis::Working, DurationUnit::Hour) => &self.hour,
            (TimeBasis::Working, DurationUnit::Day) => &self.day,
            (TimeBasis::Working, DurationUnit::Week) => &self.week,
            (TimeBasis::Working, DurationUnit::Month) => &self.month,
            (TimeBasis::Elapsed, DurationUnit::Minute) => &self.elapsed_minute,
            (TimeBasis::Elapsed, DurationUnit::Hour) => &self.elapsed_hour,
            (TimeBasis::Elapsed, DurationUnit::Day) => &self.elapsed_day,
            (TimeBasis::Elapsed, DurationUnit::Week) => &self.elapsed_week,
            (TimeBasis::Elapsed, DurationUnit::Month) => &self.elapsed_month,
        }
    }
}

/// Ordre des composantes d'une date courte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DateOrder {
    /// Jour / mois / année (français).
    #[default]
    DayMonthYear,
    /// Mois / jour / année (anglais).
    MonthDayYear,
}

/// Libellés et conventions d'affichage d'une langue.
///
/// Les tableaux se lisent par `.get()` seulement. Mois : janvier d'abord ;
/// jours : dans l'ordre de `planrs_core::Weekday::ALL` (dimanche d'abord).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormatLabels {
    /// Séparateur décimal (FR `,` ; EN `.`).
    pub decimal_sep: char,
    /// Séparateur de liste des prédécesseurs (FR `;` ; EN `,`).
    pub list_sep: String,
    /// Libellés des unités de durée.
    pub units: UnitLabels,
    /// Code du lien fin à début (FR `FD` ; EN `FS`).
    pub link_fs: String,
    /// Code du lien début à début (FR `DD` ; EN `SS`).
    pub link_ss: String,
    /// Code du lien fin à fin (`FF`).
    pub link_ff: String,
    /// Code du lien début à fin (FR `DF` ; EN `SF`).
    pub link_sf: String,
    /// Ordre des composantes d'une date.
    pub date_order: DateOrder,
    /// « Oui ».
    pub yes: String,
    /// « Non ».
    pub no: String,
    /// Texte affiché quand une valeur ne peut pas être formatée.
    pub unknown: String,
    /// Noms longs des mois, janvier d'abord.
    pub months_long: [String; 12],
    /// Noms courts des mois, janvier d'abord.
    pub months_short: [String; 12],
    /// Noms courts des jours, dimanche d'abord.
    pub weekdays_short: [String; 7],
    /// Initiales des jours, dimanche d'abord.
    pub weekday_initials: [String; 7],
    /// Préfixe de trimestre (FR `T` ; EN `Q`).
    pub quarter_prefix: String,
}

/// Durée d'une tâche dans son unité (« 5 jours », « 2,5 jours », « 1 jour? »).
///
/// Corps provisoire : chaîne vide.
#[must_use]
pub fn format_duration(_d: &Duration, _f: &ConversionFactors, _l: &FormatLabels) -> String {
    String::new()
}

/// Minutes exprimées dans une unité et une base données (« 31 jours »).
///
/// Corps provisoire : chaîne vide.
#[must_use]
pub fn format_minutes_as(
    _m: Minutes,
    _unit: DurationUnit,
    _basis: TimeBasis,
    _f: &ConversionFactors,
    _l: &FormatLabels,
) -> String {
    String::new()
}

/// Liste des prédécesseurs (« 3;5DD+2 jours ») : ID seul pour un lien fin à
/// début sans décalage, `?` pour un UID inconnu.
///
/// Corps provisoire : chaîne vide.
#[must_use]
pub fn format_predecessors(
    _links: &[Link],
    _project: &Project,
    _f: &ConversionFactors,
    _l: &FormatLabels,
) -> String {
    String::new()
}

/// Date courte (« lun 19/10/26 » / « Mon 10/19/26 »).
///
/// Corps provisoire : chaîne vide.
#[must_use]
pub fn format_date(_m: Moment, _l: &FormatLabels) -> String {
    String::new()
}

/// Marge en jours ouvrés, signée (« 0 jour », « -2 jours »).
///
/// Corps provisoire : chaîne vide.
#[must_use]
pub fn format_slack(_m: Minutes, _f: &ConversionFactors, _l: &FormatLabels) -> String {
    String::new()
}
