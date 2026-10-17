//! Calendriers (ADR-0008, § 6).
//!
//! État :
//! - [`model`] : modèle stocké (plages, jours, base ou dérivé, exceptions,
//!   récurrences, semaines datées), validé ;
//! - [`builtin`] : Standard, 24 heures, Équipe de nuit ;
//! - [`resolve`] : chaîne d'héritage ([`resolve::chain`], R-015 pour la
//!   détection des erreurs).
//!
//! À compléter avec les calendriers (agent suivant, ADR-0008 § 6.3 à 6.6,
//! règles R-015 à R-031) : `recurrence` (dépliage), `resolve::resolve_day` et
//! `resolve::expand_exception`, `compiled` (forme compilée et conversions
//! ouvrées, `CalendarSet`), `scale` (`TimeScale`, placement),
//! `Calendar::add_exception`, et le contrôle des exceptions qui se
//! chevauchent dans `Project::validate`.

pub mod builtin;
pub mod model;
pub mod resolve;

use serde::{Deserialize, Serialize};

/// Sens d'un dépassement du domaine des dates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Avant le 1er janvier 1984.
    Before,
    /// Après le 31 décembre 2149.
    After,
}

impl std::fmt::Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Before => "avant le 1er janvier 1984",
            Self::After => "après le 31 décembre 2149",
        })
    }
}

pub use builtin::{BuiltinCalendarNames, night_shift, standard, twenty_four_hours};
pub use model::{
    Calendar, CalendarException, CalendarKind, DayHours, DayRule, DaySetting, MonthItem, MonthPosition,
    Recurrence, TimeRange, WorkWeek,
};
pub use resolve::{CalendarChain, chain};
