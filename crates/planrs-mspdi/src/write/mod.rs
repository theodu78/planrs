//! Écriture MSPDI (R-120, R-121, R-127 à R-132 ; ADR-0010, D11).
//!
//! [`write`] est infaillible (règle 10) : toute anomalie (valeur arrondie,
//! chaîne tronquée, caractère interdit) devient un [`WriteDiagnostic`].
//! L'ordre des éléments vient de la table du schéma ([`crate::schema`]) :
//! profil [`OrderProfile::Observed`] (défaut, ordre des fichiers
//! enregistrés par MS Project, O19) ou [`OrderProfile::Strict`] (ordre pj15,
//! sans `GUID` ni élément hors schéma). Tout champ modélisé non optionnel
//! est écrit explicitement, même égal au défaut du schéma : la relecture ne
//! dépend jamais des défauts (ADR-0008, R-076). Sortie déterministe octet
//! pour octet (aucune `HashMap`, ordre des arènes).
//!
//! Aucune E/S : la sortie est en octets (UTF-8 sans BOM).

mod calendar;
mod computed;
mod emit;
mod project;
mod task;

use planrs_core::{Percent, Project, Subject, round_div};

pub use crate::schema::Profile as OrderProfile;
use crate::schema::{Parent, sequence};
pub use computed::{Computed, ScheduleGap, choose_computed, schedule_gaps};
use emit::Emitter;

/// Fin de ligne.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Newline {
    /// `\n` (défaut).
    #[default]
    Lf,
    /// `\r\n`.
    Crlf,
}

/// Indentation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Indent {
    /// Une tabulation par niveau (défaut, comme MS Project).
    #[default]
    Tab,
    /// N espaces par niveau.
    Spaces(u8),
    /// Aucune indentation ni fin de ligne entre les éléments.
    None,
}

/// Options d'écriture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteOptions {
    /// Profil d'ordre (R-121).
    pub profile: OrderProfile,
    /// `SaveVersion` (R-130) : 14, version écrite par MS Project 2010 et
    /// suivants.
    pub save_version: i64,
    /// `CurrencyCode` écrit si le projet n'en a pas relu un (R-130).
    pub currency_code: String,
    /// Fin de ligne.
    pub newline: Newline,
    /// Indentation.
    pub indent: Indent,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            profile: OrderProfile::Observed,
            save_version: 14,
            currency_code: "EUR".to_owned(),
            newline: Newline::Lf,
            indent: Indent::Tab,
        }
    }
}

/// Document écrit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Written {
    /// Octets UTF-8, sans BOM.
    pub xml: Vec<u8>,
    /// Diagnostics d'écriture, dans l'ordre d'écriture.
    pub diagnostics: Vec<WriteDiagnostic>,
}

/// Diagnostic d'écriture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteDiagnostic {
    /// Nature.
    pub code: WriteCode,
    /// Élément du projet concerné.
    pub subject: Option<Subject>,
    /// Chemin de l'élément XML (« Tasks/Task/Name »).
    pub element: String,
    /// Valeur en cause (tronquée à 200 caractères).
    pub value: Option<String>,
}

/// Nature d'un diagnostic d'écriture (codes stables `mspdi.write.*`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteCode {
    /// Valeur arrondie à la précision du format (pourcentage entier…).
    Rounded,
    /// Chaîne tronquée au `maxLength` du schéma (en caractères).
    Truncated {
        /// Longueur maximale.
        max: u32,
    },
    /// Caractère interdit par XML 1.0 remplacé par U+FFFD.
    InvalidChar,
    /// Le `Schedule` ne correspond pas aux tâches du projet (autre projet,
    /// projet modifié) : aucune valeur calculée écrite.
    ScheduleMismatch,
    /// Planning de planrs incomplet pour ce projet et aucune valeur relue :
    /// valeurs calculées omises (R-122).
    ScheduleIncomplete {
        /// Manques.
        gaps: Vec<ScheduleGap>,
    },
}

impl WriteCode {
    /// Code stable.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Rounded => "mspdi.write.rounded",
            Self::Truncated { .. } => "mspdi.write.truncated",
            Self::InvalidChar => "mspdi.write.invalid_char",
            Self::ScheduleMismatch => "mspdi.write.schedule_mismatch",
            Self::ScheduleIncomplete { .. } => "mspdi.write.schedule_incomplete",
        }
    }
}

/// Écrit un projet en MSPDI, avec les valeurs calculées de `computed`
/// (R-122 ; [`choose_computed`] choisit la source). Infaillible : toute
/// anomalie devient un diagnostic.
#[must_use]
pub fn write(project: &Project, computed: Computed<'_>, options: &WriteOptions) -> Written {
    let mut em = Emitter::new(options.newline, options.indent);
    let values = match computed {
        Computed::Schedule(s) if computed::matches(project, s) => Some(computed::from_schedule(project, s)),
        Computed::Schedule(_) => {
            em.report(WriteCode::ScheduleMismatch, "Tasks", None);
            None
        }
        Computed::Recorded(r) => Some(computed::from_recorded(r)),
        Computed::Omit => None,
    };
    em.root(&[]);
    project::write(&mut em, project, values.as_ref(), options);
    em.end_root();
    let (xml, diagnostics) = em.finish();
    Written { xml, diagnostics }
}

/// Écrit une entité de `parent` sous l'élément `element` : pour chaque nom
/// du profil, `field` écrit la valeur modélisée, déduite ou calculée.
fn entity(
    em: &mut Emitter,
    element: &'static str,
    parent: Parent,
    profile: OrderProfile,
    mut field: impl FnMut(&mut Emitter, &'static str),
) {
    em.open(element);
    for name in sequence(parent).names(profile) {
        field(em, name);
    }
    em.close();
}

/// Longueur maximale d'un élément de `parent` (table du schéma).
fn max_len(parent: Parent, name: &str) -> Option<u16> {
    sequence(parent).slot(name).and_then(|s| s.max_len)
}

/// R-127 : pourcentage entier `round_div(p, 100)` (R-001), arrondi signalé.
fn percent(em: &mut Emitter, name: &'static str, p: Percent) {
    let v = p.get();
    let whole = round_div(i128::from(v), 100).unwrap_or(0);
    if v.checked_rem(100).is_some_and(|r| r != 0) {
        em.report(WriteCode::Rounded, name, Some(v.to_string()));
    }
    em.leaf(name, &whole.to_string());
}
