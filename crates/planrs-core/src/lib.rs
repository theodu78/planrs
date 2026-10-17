//! Cœur de planrs : modèle de données et moteur de planification.
//!
//! Cette crate ne fait aucune entrée/sortie : elle ne connaît ni le système de
//! fichiers, ni l'heure système, ni MCP, ni egui (règles 5 et 6 de `CLAUDE.md`).
//! « Maintenant » est toujours injecté par l'appelant. Les durées sont en
//! minutes entières (`i64`) et aucun flottant n'est admis dans le moteur.
//! Elle compile en `wasm32-unknown-unknown` (vérifié en CI).
//!
//! Gardes, en plus des lints du workspace :
//! - les attributs ci-dessous (calcul flottant, sorties console) ;
//! - `clippy.toml` de cette crate : horloges (`chrono::Utc::now`,
//!   `SystemTime::now`…), système de fichiers (`std::fs`), variables
//!   d'environnement, types `f32`/`f64`, `HashMap`/`HashSet`. Il s'applique
//!   aussi dans un build du workspace, où chrono reçoit la feature `now` par
//!   rmcp.
//!
//! État : jalon J1 en cours (ADR-0008). Modules :
//! - [`fixed`] : arrondi unique (R-001) et décimal exact ;
//! - [`quantity`] : unités, pourcentages et coûts en point fixe (R-002) ;
//! - [`time`] : instants à la minute, jours, heures, semaines (R-003 à R-005) ;
//! - [`duration`] : minutes, durées, travail, décalages, facteurs de
//!   conversion des unités (R-006 à R-010) ;
//! - [`ids`] : UID, ID, GUID, numéros de ligne de base ;
//! - [`arena`] : collections ordonnées indexées par UID ;
//! - [`calendar`] : modèle stocké des calendriers, calendriers fournis,
//!   résolution et récurrences, forme compilée et conversions ouvrées,
//!   placement des durées et des jalons (R-011 à R-031) ;
//! - [`model`] : modèle V1 (projet, tâches, liens, ressources, affectations,
//!   lignes de base), sérialisation validée (R-032 à R-034) ;
//! - [`diagnostic`] : diagnostics typés, gravité et sujet déduits ;
//! - [`error`] : erreurs typées, codes stables.

#![deny(
    clippy::float_arithmetic,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro
)]

pub mod arena;
pub mod calendar;
pub mod diagnostic;
pub mod duration;
pub mod error;
pub mod fixed;
pub mod ids;
pub mod model;
pub mod parity;
pub mod quantity;
pub mod time;

pub use arena::{Arena, HasUid};
pub use calendar::{BuiltinCalendarNames, CalendarSet, CompiledCalendar, Direction, Span, TimeScale};
pub use diagnostic::{Diagnostic, Diagnostics, Pass, Severity, Subject};
pub use duration::{
    ConversionFactors, Duration, DurationUnit, Lag, LagValue, Minutes, TimeBasis, Work, percent_of,
};
pub use error::{
    CalendarError, CalendarModelError, CoreError, DurationError, EntityKind, FixedError, ModelError,
    TimeError,
};
pub use fixed::{Decimal, round_div};
pub use ids::{AssignmentUid, BaselineNumber, CalendarUid, EntityUid, Guid, ResourceUid, TaskId, TaskUid};
pub use model::{
    Assignment, AssignmentBaseline, Constraint, ExtraEntry, Extras, Link, LinkType, OutlineLevel, Priority,
    Progress, Project, ProjectInfo, ProjectInit, ProjectOptions, Resource, ResourceBaseline, ResourceKind,
    ScheduleAnchor, SchedulingMode, Task, TaskBaseline, TaskType, WorkContour,
};
pub use quantity::{Money, Percent, Rate, RateUnit, SignedPercent, Units};
pub use time::{Day, DayRange, Moment, Month, TimeOfDay, Week, Weekday};

/// Nom affiché du produit.
///
/// Seul endroit qui définit ce nom dans le code : la CLI, la fenêtre desktop
/// et la version web le lisent ici. Le nom `planrs` est provisoire ; les autres
/// endroits où il apparaît (noms de crates, binaires, fichiers) sont listés
/// dans `docs/rename.md`.
pub const PRODUCT_NAME: &str = "planrs";
