//! Calendriers (ADR-0008, § 6).
//!
//! État :
//! - [`model`] : modèle stocké (plages, jours, base ou dérivé, exceptions,
//!   récurrences, semaines datées), validé ;
//! - [`builtin`] : Standard, 24 heures, Équipe de nuit ;
//! - [`recurrence`] : dépliage des récurrences (R-018) ;
//! - [`resolve`] : chaîne d'héritage ([`resolve::chain`], R-015) et
//!   résolution d'un jour ([`resolve::resolve_day`], R-016, R-017, R-019),
//!   chemin froid.
//!
//! À venir : `compiled` (forme compilée et conversions ouvrées,
//! `CalendarSet`), `scale` (`TimeScale`, placement).

pub mod builtin;
pub mod model;
pub mod recurrence;
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
pub use recurrence::expand_exception;
pub use resolve::{CalendarChain, DaySource, ResolvedDay, chain, resolve_day};
