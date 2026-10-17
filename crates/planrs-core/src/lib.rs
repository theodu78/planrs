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
//! - [`error`] : erreurs typées, codes stables.

#![deny(
    clippy::float_arithmetic,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::dbg_macro
)]

pub mod error;
pub mod fixed;
pub mod parity;
pub mod quantity;

pub use error::FixedError;
pub use fixed::{Decimal, round_div};
pub use quantity::{Money, Percent, Rate, RateUnit, SignedPercent, Units};

/// Nom affiché du produit.
///
/// Seul endroit qui définit ce nom dans le code : la CLI, la fenêtre desktop
/// et la version web le lisent ici. Le nom `planrs` est provisoire ; les autres
/// endroits où il apparaît (noms de crates, binaires, fichiers) sont listés
/// dans `docs/rename.md`.
pub const PRODUCT_NAME: &str = "planrs";
