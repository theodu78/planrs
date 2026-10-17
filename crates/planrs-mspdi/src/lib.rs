//! Import et export du format MSPDI XML (Microsoft Project Data Interchange).
//!
//! MSPDI est le format natif de planrs en V1 : il garantit l'échange avec
//! MS Project. L'implémentation suit le schéma XML public de Microsoft
//! (`mspdi_pj15.xsd`, Project 2013 SDK, cité par URL et empreinte, jamais
//! commité) et le comportement observé sur des fichiers enregistrés par
//! MS Project (clean-room, règle 1). Aucun flottant : les décimales des
//! durées sont lues en entiers et arrondies par la règle unique R-001.
//!
//! État (J2, ADR-0009) : lecture seule, en cours.
//! - [`scalar`] : booléens, entiers, dates, heures, durées ISO 8601,
//!   dixièmes de minute (R-070, R-072, R-073, R-075) ;
//! - [`codes`] : tables des codes numériques MSPDI vers les types du cœur.

pub mod codes;
pub mod scalar;
