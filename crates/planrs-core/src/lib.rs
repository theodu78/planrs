//! Cœur de planrs : modèle de données et moteur de planification.
//!
//! Cette crate ne fait aucune entrée/sortie : elle ne connaît ni le système de
//! fichiers, ni l'heure système, ni MCP, ni egui (règles 5 et 6 de `CLAUDE.md`).
//! « Maintenant » est toujours injecté par l'appelant. Les durées sont en
//! minutes entières (`i64`) et aucun flottant n'est admis dans le moteur : la
//! lint `clippy::float_arithmetic` est interdite ici en plus des lints du
//! workspace. Elle compile en `wasm32-unknown-unknown` (vérifié en CI).
//!
//! État : squelette du jalon J1, pas encore de modèle.

#![deny(clippy::float_arithmetic)]

pub mod parity;

/// Nom affiché du produit.
///
/// Seul endroit qui définit ce nom dans le code : la CLI, la fenêtre desktop
/// et la version web le lisent ici. Le nom `planrs` est provisoire ; les autres
/// endroits où il apparaît (noms de crates, binaires, fichiers) sont listés
/// dans `docs/rename.md`.
pub const PRODUCT_NAME: &str = "planrs";
