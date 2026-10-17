//! Calendriers (ADR-0008, § 6).
//!
//! État :
//! - [`model`] : modèle stocké (plages, jours, base ou dérivé, exceptions,
//!   récurrences, semaines datées), validé ;
//! - [`builtin`] : Standard, 24 heures, Équipe de nuit.
//!
//! À compléter avec les calendriers (agent suivant, ADR-0008 § 6.3 à 6.6,
//! règles R-011 à R-031) : `recurrence` (dépliage), `resolve` (résolution d'un
//! jour, au-delà de la chaîne d'héritage), `compiled` (forme compilée et
//! conversions ouvrées, `CalendarSet`), `scale` (`TimeScale`, placement).

pub mod builtin;
pub mod model;

pub use builtin::{BuiltinCalendarNames, night_shift, standard, twenty_four_hours};
pub use model::{
    Calendar, CalendarException, CalendarKind, DayHours, DayRule, DaySetting, MonthItem, MonthPosition,
    Recurrence, TimeRange, WorkWeek,
};
