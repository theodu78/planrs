//! Traductions : identifiants typés ([`TextId`]), fichiers fluent embarqués
//! et choix de la langue.
//!
//! Le code ne passe jamais une clé libre : seulement un [`TextId`], généré
//! avec sa clé par la macro `text_ids!`. Un test vérifie que les deux
//! fichiers `locales/fr.ftl` et `locales/en.ftl` déclarent exactement les clés
//! de [`TextId::ALL`]. Repli : anglais → français → clé brute (journalisée,
//! jamais de panique). L'isolation bidirectionnelle de fluent est coupée
//! (`set_use_isolating(false)`) : ses caractères U+2068/U+2069 manquent aux
//! polices d'egui. Les nombres décimaux et les dates sont pré-formatés par
//! l'interface : fluent ne reçoit que des chaînes et des entiers.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeSet;

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::{LanguageIdentifier, langid};

use crate::format::{DateOrder, FormatLabels, UnitLabel, UnitLabels};
use crate::state::LanguagePref;

/// Source fluent française (langue de repli).
const FTL_FR: &str = include_str!("../locales/fr.ftl");
/// Source fluent anglaise.
const FTL_EN: &str = include_str!("../locales/en.ftl");

/// Langue d'affichage résolue (non sérialisée : l'état persiste la
/// préférence, `LanguagePref`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Language {
    /// Français (langue de repli).
    Fr,
    /// Anglais.
    En,
}

impl Language {
    /// Toutes les langues livrées.
    pub const ALL: [Self; 2] = [Self::Fr, Self::En];

    /// Identifiant de langue fluent.
    #[must_use]
    pub fn langid(self) -> LanguageIdentifier {
        match self {
            Self::Fr => langid!("fr-FR"),
            Self::En => langid!("en-US"),
        }
    }

    /// Langue d'une étiquette système (« fr », « fr-CA », « en_US.UTF-8 ») ;
    /// `None` pour une langue non livrée (« de-DE », « C »).
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        let primary = tag.trim().split(['-', '_', '.', '@']).next().unwrap_or_default();
        if primary.eq_ignore_ascii_case("fr") {
            Some(Self::Fr)
        } else if primary.eq_ignore_ascii_case("en") {
            Some(Self::En)
        } else {
            None
        }
    }

    /// Langue du système (bureau : réglages régionaux ; web :
    /// `navigator.language`), si elle est livrée. À appeler par
    /// l'application, jamais par les tests.
    #[must_use]
    pub fn from_system() -> Option<Self> {
        sys_locale::get_locale().and_then(|tag| Self::from_tag(&tag))
    }

    /// Langue effective : la préférence, sinon la langue du système, sinon le
    /// français.
    #[must_use]
    pub fn resolve(pref: LanguagePref, system: Option<Self>) -> Self {
        match pref {
            LanguagePref::Fr => Self::Fr,
            LanguagePref::En => Self::En,
            LanguagePref::System => system.unwrap_or(Self::Fr),
        }
    }

    /// Source fluent embarquée de la langue.
    const fn source(self) -> &'static str {
        match self {
            Self::Fr => FTL_FR,
            Self::En => FTL_EN,
        }
    }
}

/// Génère [`TextId`], sa liste [`TextId::ALL`] et sa méthode `key()`.
macro_rules! text_ids {
    ($($variant:ident => $key:literal),* $(,)?) => {
        /// Identifiant d'un message traduit (clé fluent vérifiée à la
        /// compilation).
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum TextId {
            $(
                #[doc = concat!("Message `", $key, "`.")]
                $variant,
            )*
        }

        impl TextId {
            /// Tous les identifiants, dans l'ordre de déclaration.
            pub const ALL: &'static [Self] = &[$(Self::$variant),*];

            /// Clé fluent du message.
            #[must_use]
            pub const fn key(self) -> &'static str {
                match self {
                    $(Self::$variant => $key,)*
                }
            }
        }
    };
}

text_ids! {
    // Écran d'accueil.
    WelcomeTitle => "welcome-title",
    WelcomeSubtitle => "welcome-subtitle",
    WelcomeOpenDemo => "welcome-open-demo",
    WelcomeOpenFile => "welcome-open-file",
    WelcomeWebNote => "welcome-web-note",
    WelcomeLanguage => "welcome-language",
    WelcomeTheme => "welcome-theme",
    // Menus.
    MenuFile => "menu-file",
    MenuOpen => "menu-open",
    MenuOpenDemo => "menu-open-demo",
    MenuClose => "menu-close",
    MenuView => "menu-view",
    MenuZoom => "menu-zoom",
    MenuTheme => "menu-theme",
    MenuLanguage => "menu-language",
    // Barre d'outils et zoom.
    ToolbarFit => "toolbar-fit",
    ZoomDay => "zoom-day",
    ZoomWeek => "zoom-week",
    ZoomMonth => "zoom-month",
    ZoomQuarter => "zoom-quarter",
    // Thème et langue.
    ThemeSystem => "theme-system",
    ThemeLight => "theme-light",
    ThemeDark => "theme-dark",
    LangSystem => "lang-system",
    LangFr => "lang-fr",
    LangEn => "lang-en",
    // Colonnes.
    ColId => "col-id",
    ColName => "col-name",
    ColDuration => "col-duration",
    ColStart => "col-start",
    ColFinish => "col-finish",
    ColPredecessors => "col-predecessors",
    ColTotalSlack => "col-total-slack",
    ColCritical => "col-critical",
    Yes => "yes",
    No => "no",
    // Unités de durée.
    UnitMinuteOne => "unit-minute-one",
    UnitMinuteOther => "unit-minute-other",
    UnitHourOne => "unit-hour-one",
    UnitHourOther => "unit-hour-other",
    UnitDayOne => "unit-day-one",
    UnitDayOther => "unit-day-other",
    UnitWeekOne => "unit-week-one",
    UnitWeekOther => "unit-week-other",
    UnitMonthOne => "unit-month-one",
    UnitMonthOther => "unit-month-other",
    UnitMinuteElapsedOne => "unit-minute-elapsed-one",
    UnitMinuteElapsedOther => "unit-minute-elapsed-other",
    UnitHourElapsedOne => "unit-hour-elapsed-one",
    UnitHourElapsedOther => "unit-hour-elapsed-other",
    UnitDayElapsedOne => "unit-day-elapsed-one",
    UnitDayElapsedOther => "unit-day-elapsed-other",
    UnitWeekElapsedOne => "unit-week-elapsed-one",
    UnitWeekElapsedOther => "unit-week-elapsed-other",
    UnitMonthElapsedOne => "unit-month-elapsed-one",
    UnitMonthElapsedOther => "unit-month-elapsed-other",
    // Types de liens.
    LinkFs => "link-fs",
    LinkSs => "link-ss",
    LinkFf => "link-ff",
    LinkSf => "link-sf",
    // Conventions de formatage.
    FmtDecimalSep => "fmt-decimal-sep",
    FmtListSep => "fmt-list-sep",
    FmtDateOrder => "fmt-date-order",
    FmtUnknown => "fmt-unknown",
    // Mois.
    Month1Long => "month-1-long",
    Month1Short => "month-1-short",
    Month2Long => "month-2-long",
    Month2Short => "month-2-short",
    Month3Long => "month-3-long",
    Month3Short => "month-3-short",
    Month4Long => "month-4-long",
    Month4Short => "month-4-short",
    Month5Long => "month-5-long",
    Month5Short => "month-5-short",
    Month6Long => "month-6-long",
    Month6Short => "month-6-short",
    Month7Long => "month-7-long",
    Month7Short => "month-7-short",
    Month8Long => "month-8-long",
    Month8Short => "month-8-short",
    Month9Long => "month-9-long",
    Month9Short => "month-9-short",
    Month10Long => "month-10-long",
    Month10Short => "month-10-short",
    Month11Long => "month-11-long",
    Month11Short => "month-11-short",
    Month12Long => "month-12-long",
    Month12Short => "month-12-short",
    // Jours de la semaine.
    WeekdayMonShort => "weekday-mon-short",
    WeekdayMonInitial => "weekday-mon-initial",
    WeekdayTueShort => "weekday-tue-short",
    WeekdayTueInitial => "weekday-tue-initial",
    WeekdayWedShort => "weekday-wed-short",
    WeekdayWedInitial => "weekday-wed-initial",
    WeekdayThuShort => "weekday-thu-short",
    WeekdayThuInitial => "weekday-thu-initial",
    WeekdayFriShort => "weekday-fri-short",
    WeekdayFriInitial => "weekday-fri-initial",
    WeekdaySatShort => "weekday-sat-short",
    WeekdaySatInitial => "weekday-sat-initial",
    WeekdaySunShort => "weekday-sun-short",
    WeekdaySunInitial => "weekday-sun-initial",
    // Échelle de temps.
    TierQuarter => "tier-quarter",
    // Barre d'état.
    StatusTasks => "status-tasks",
    StatusRange => "status-range",
    StatusErrors => "status-errors",
    StatusWarnings => "status-warnings",
    StatusNoDocument => "status-no-document",
    // Ouverture.
    OpenInProgress => "open-in-progress",
    OpenErrorTitle => "open-error-title",
    OpenErrorRead => "open-error-read",
    OpenErrorMspdi => "open-error-mspdi",
    // Accessibilité et divers.
    RowLabel => "row-label",
    SplitterLabel => "splitter-label",
    GanttScrollLabel => "gantt-scroll-label",
    DemoFileName => "demo-file-name",
    FileFilterMspdi => "file-filter-mspdi",
    DialogOpenTitle => "dialog-open-title",
    StateLoadWarning => "state-load-warning",
}

/// Noms longs des mois, janvier d'abord.
const MONTHS_LONG: [TextId; 12] = [
    TextId::Month1Long,
    TextId::Month2Long,
    TextId::Month3Long,
    TextId::Month4Long,
    TextId::Month5Long,
    TextId::Month6Long,
    TextId::Month7Long,
    TextId::Month8Long,
    TextId::Month9Long,
    TextId::Month10Long,
    TextId::Month11Long,
    TextId::Month12Long,
];

/// Noms courts des mois, janvier d'abord.
const MONTHS_SHORT: [TextId; 12] = [
    TextId::Month1Short,
    TextId::Month2Short,
    TextId::Month3Short,
    TextId::Month4Short,
    TextId::Month5Short,
    TextId::Month6Short,
    TextId::Month7Short,
    TextId::Month8Short,
    TextId::Month9Short,
    TextId::Month10Short,
    TextId::Month11Short,
    TextId::Month12Short,
];

/// Noms courts des jours, dimanche d'abord (ordre de `Weekday::ALL`).
const WEEKDAYS_SHORT: [TextId; 7] = [
    TextId::WeekdaySunShort,
    TextId::WeekdayMonShort,
    TextId::WeekdayTueShort,
    TextId::WeekdayWedShort,
    TextId::WeekdayThuShort,
    TextId::WeekdayFriShort,
    TextId::WeekdaySatShort,
];

/// Initiales des jours, dimanche d'abord (ordre de `Weekday::ALL`).
const WEEKDAY_INITIALS: [TextId; 7] = [
    TextId::WeekdaySunInitial,
    TextId::WeekdayMonInitial,
    TextId::WeekdayTueInitial,
    TextId::WeekdayWedInitial,
    TextId::WeekdayThuInitial,
    TextId::WeekdayFriInitial,
    TextId::WeekdaySatInitial,
];

/// Argument d'un message : chaîne pré-formatée ou entier (pluriels).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arg<'a> {
    /// Chaîne déjà formatée (nom, date, durée…).
    Str(Cow<'a, str>),
    /// Entier, pour la sélection plurielle et l'affichage de compteurs.
    Count(i64),
}

/// Messages d'une langue, avec repli sur le français.
pub struct Texts {
    /// Langue demandée.
    language: Language,
    /// Messages de la langue demandée.
    bundle: FluentBundle<FluentResource>,
    /// Messages français, quand la langue demandée n'est pas le français.
    fallback: Option<FluentBundle<FluentResource>>,
    /// Identifiants déjà signalés comme manquants (journalisés une fois).
    missing: RefCell<BTreeSet<TextId>>,
}

impl std::fmt::Debug for Texts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texts")
            .field("language", &self.language)
            .finish_non_exhaustive()
    }
}

/// Construit le recueil fluent d'une langue. Ne rate jamais : les erreurs
/// d'analyse sont journalisées et la ressource partielle est utilisée.
fn build_bundle(language: Language) -> FluentBundle<FluentResource> {
    let mut bundle = FluentBundle::new(vec![language.langid()]);
    bundle.set_use_isolating(false);
    let resource = match FluentResource::try_new(language.source().to_owned()) {
        Ok(resource) => resource,
        Err((resource, errors)) => {
            for error in errors {
                log::error!("traductions {language:?} : erreur d'analyse : {error:?}");
            }
            resource
        }
    };
    if let Err(errors) = bundle.add_resource(resource) {
        for error in errors {
            log::error!("traductions {language:?} : {error}");
        }
    }
    bundle
}

impl Texts {
    /// Charge les messages d'une langue (et le français en repli).
    #[must_use]
    pub fn new(language: Language) -> Self {
        let fallback = (language != Language::Fr).then(|| build_bundle(Language::Fr));
        Self {
            language,
            bundle: build_bundle(language),
            fallback,
            missing: RefCell::new(BTreeSet::new()),
        }
    }

    /// Langue des messages.
    #[must_use]
    pub const fn language(&self) -> Language {
        self.language
    }

    /// Message sans argument.
    #[must_use]
    pub fn get(&self, id: TextId) -> String {
        self.format(id, None)
    }

    /// Message avec arguments nommés.
    #[must_use]
    pub fn get_args(&self, id: TextId, args: &[(&str, Arg<'_>)]) -> String {
        let mut fluent_args = FluentArgs::new();
        for (name, arg) in args {
            let value = match arg {
                Arg::Str(text) => FluentValue::from(text.as_ref()),
                Arg::Count(count) => FluentValue::from(*count),
            };
            fluent_args.set(*name, value);
        }
        self.format(id, Some(&fluent_args))
    }

    /// Recherche dans la langue demandée, puis en français, puis rend la clé.
    fn format(&self, id: TextId, args: Option<&FluentArgs<'_>>) -> String {
        let key = id.key();
        for bundle in std::iter::once(&self.bundle).chain(self.fallback.as_ref()) {
            if let Some(pattern) = bundle.get_message(key).and_then(|message| message.value()) {
                let mut errors = Vec::new();
                let text = bundle.format_pattern(pattern, args, &mut errors);
                for error in errors {
                    log::warn!("traduction « {key} » : {error}");
                }
                return text.into_owned();
            }
        }
        if self.missing.borrow_mut().insert(id) {
            log::warn!("traduction manquante : « {key} » ({:?})", self.language);
        }
        key.to_owned()
    }

    /// Libellé d'unité (singulier, pluriel).
    fn unit(&self, one: TextId, other: TextId) -> UnitLabel {
        UnitLabel {
            one: self.get(one),
            other: self.get(other),
        }
    }

    /// Libellés et conventions des formateurs, résolus une fois pour la
    /// langue.
    #[must_use]
    pub fn format_labels(&self) -> FormatLabels {
        let date_order = if self.get(TextId::FmtDateOrder).trim() == "mdy" {
            DateOrder::MonthDayYear
        } else {
            DateOrder::DayMonthYear
        };
        FormatLabels {
            decimal_sep: self.get(TextId::FmtDecimalSep).chars().next().unwrap_or(','),
            list_sep: self.get(TextId::FmtListSep),
            units: UnitLabels {
                minute: self.unit(TextId::UnitMinuteOne, TextId::UnitMinuteOther),
                hour: self.unit(TextId::UnitHourOne, TextId::UnitHourOther),
                day: self.unit(TextId::UnitDayOne, TextId::UnitDayOther),
                week: self.unit(TextId::UnitWeekOne, TextId::UnitWeekOther),
                month: self.unit(TextId::UnitMonthOne, TextId::UnitMonthOther),
                elapsed_minute: self.unit(TextId::UnitMinuteElapsedOne, TextId::UnitMinuteElapsedOther),
                elapsed_hour: self.unit(TextId::UnitHourElapsedOne, TextId::UnitHourElapsedOther),
                elapsed_day: self.unit(TextId::UnitDayElapsedOne, TextId::UnitDayElapsedOther),
                elapsed_week: self.unit(TextId::UnitWeekElapsedOne, TextId::UnitWeekElapsedOther),
                elapsed_month: self.unit(TextId::UnitMonthElapsedOne, TextId::UnitMonthElapsedOther),
            },
            link_fs: self.get(TextId::LinkFs),
            link_ss: self.get(TextId::LinkSs),
            link_ff: self.get(TextId::LinkFf),
            link_sf: self.get(TextId::LinkSf),
            date_order,
            yes: self.get(TextId::Yes),
            no: self.get(TextId::No),
            unknown: self.get(TextId::FmtUnknown),
            months_long: MONTHS_LONG.map(|id| self.get(id)),
            months_short: MONTHS_SHORT.map(|id| self.get(id)),
            weekdays_short: WEEKDAYS_SHORT.map(|id| self.get(id)),
            weekday_initials: WEEKDAY_INITIALS.map(|id| self.get(id)),
            quarter_prefix: self.get(TextId::TierQuarter),
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

    use fluent_syntax::ast::Entry;

    use super::{Arg, FTL_EN, FTL_FR, Language, TextId, Texts};
    use crate::format::DateOrder;
    use crate::state::LanguagePref;

    /// Clés des messages d'une source fluent ; échoue sur toute erreur
    /// d'analyse.
    fn message_keys(source: &str) -> BTreeSet<String> {
        let resource = match fluent_syntax::parser::parse(source) {
            Ok(resource) => resource,
            Err((_, errors)) => panic!("erreurs d'analyse : {errors:?}"),
        };
        let mut keys = BTreeSet::new();
        for entry in resource.body {
            if let Entry::Message(message) = entry {
                assert!(
                    message.value.is_some(),
                    "message sans valeur : {}",
                    message.id.name
                );
                assert!(
                    keys.insert(message.id.name.to_owned()),
                    "clé en double : {}",
                    message.id.name
                );
            }
        }
        keys
    }

    /// Arguments d'exemple couvrant toutes les variables des messages.
    fn sample_args() -> Vec<(&'static str, Arg<'static>)> {
        vec![
            ("product", Arg::Str("planrs".into())),
            ("count", Arg::Count(3)),
            ("start", Arg::Str("lun 02/11/26".into())),
            ("finish", Arg::Str("lun 17/05/27".into())),
            ("detail", Arg::Str("détail".into())),
            ("id", Arg::Str("3".into())),
            ("name", Arg::Str("Plans".into())),
        ]
    }

    #[test]
    fn i18n_keys_match_text_ids() {
        let expected: BTreeSet<String> = TextId::ALL.iter().map(|id| id.key().to_owned()).collect();
        assert_eq!(expected.len(), TextId::ALL.len(), "clé en double dans text_ids!");
        assert_eq!(message_keys(FTL_FR), expected, "fr.ftl ≠ TextId::ALL");
        assert_eq!(message_keys(FTL_EN), expected, "en.ftl ≠ TextId::ALL");
    }

    #[test]
    fn i18n_no_isolation_marks() {
        for language in Language::ALL {
            let texts = Texts::new(language);
            for &id in TextId::ALL {
                let text = texts.get_args(id, &sample_args());
                assert!(
                    !text.contains(['\u{2068}', '\u{2069}']),
                    "{language:?} {id:?} : {text:?}"
                );
                assert!(!text.is_empty(), "{language:?} {id:?} vide");
            }
        }
    }

    #[test]
    fn i18n_no_narrow_nbsp() {
        assert!(!FTL_FR.contains('\u{202F}'));
        assert!(!FTL_EN.contains('\u{202F}'));
        for language in Language::ALL {
            let texts = Texts::new(language);
            for &id in TextId::ALL {
                assert!(
                    !texts.get_args(id, &sample_args()).contains('\u{202F}'),
                    "{language:?} {id:?}"
                );
            }
        }
    }

    #[test]
    fn i18n_plurals_and_args() {
        let fr = Texts::new(Language::Fr);
        let en = Texts::new(Language::En);
        assert_eq!(
            fr.get_args(TextId::StatusTasks, &[("count", Arg::Count(49))]),
            "49 tâches"
        );
        assert_eq!(
            fr.get_args(TextId::StatusTasks, &[("count", Arg::Count(1))]),
            "1 tâche"
        );
        assert_eq!(
            en.get_args(TextId::StatusTasks, &[("count", Arg::Count(1))]),
            "1 task"
        );
        assert_eq!(
            en.get_args(TextId::StatusErrors, &[("count", Arg::Count(0))]),
            "no errors"
        );
        assert_eq!(
            fr.get_args(TextId::StatusErrors, &[("count", Arg::Count(0))]),
            "aucune erreur"
        );
        assert_eq!(en.get(TextId::MenuFile), "File");
        assert_eq!(fr.get(TextId::MenuFile), "Fichier");
    }

    #[test]
    fn i18n_format_labels() {
        let fr = Texts::new(Language::Fr).format_labels();
        assert_eq!(fr.decimal_sep, ',');
        assert_eq!(fr.list_sep, ";");
        assert_eq!(fr.date_order, DateOrder::DayMonthYear);
        assert_eq!(fr.link_fs, "FD");
        assert_eq!(fr.months_short[0], "janv.");
        assert_eq!(fr.weekdays_short[1], "lun");
        assert_eq!(fr.units.day.other, "jours");
        let en = Texts::new(Language::En).format_labels();
        assert_eq!(en.decimal_sep, '.');
        assert_eq!(en.list_sep, ",");
        assert_eq!(en.date_order, DateOrder::MonthDayYear);
        assert_eq!(en.link_sf, "SF");
        assert_eq!(en.months_short[1], "Feb");
        assert_eq!(en.weekdays_short[0], "Sun");
    }

    #[test]
    fn language_from_tag() {
        assert_eq!(Language::from_tag("fr"), Some(Language::Fr));
        assert_eq!(Language::from_tag("fr-CA"), Some(Language::Fr));
        assert_eq!(Language::from_tag("en_US.UTF-8"), Some(Language::En));
        assert_eq!(Language::from_tag("EN"), Some(Language::En));
        assert_eq!(Language::from_tag("de"), None);
        assert_eq!(Language::from_tag("C"), None);
        assert_eq!(Language::from_tag(""), None);
    }

    #[test]
    fn language_resolve() {
        assert_eq!(Language::resolve(LanguagePref::System, None), Language::Fr);
        assert_eq!(
            Language::resolve(LanguagePref::System, Some(Language::En)),
            Language::En
        );
        assert_eq!(
            Language::resolve(LanguagePref::Fr, Some(Language::En)),
            Language::Fr
        );
        assert_eq!(Language::resolve(LanguagePref::En, None), Language::En);
    }
}
