//! Session « headless » de planrs : projet ouvert, historique et registre de
//! commandes.
//!
//! Toute action (menu, raccourci, CLI, MCP, canal de contrôle) passe par le
//! même registre de commandes, identifiées par un id stable (`task.add`,
//! `link.add`…). Cette crate ne dépend d'aucune interface graphique
//! (règle 9, vérifiée par `cargo xtask layers`).
//!
//! État : le registre arrive en J4. Modules livrés en J3 (ADR-0010) :
//! - [`text`] : texte des durées, des décalages, des prédécesseurs et des
//!   dates à la manière de MS Project, en français et en anglais (R-140 à
//!   R-143) ;
//! - [`csv`] : import des tâches depuis un CSV de MS Project ou d'un
//!   tableur (R-144, R-145).
//!
//! Aucune entrée/sortie dans ces modules : du texte ou des octets en entrée,
//! une valeur du modèle en sortie.

pub mod csv;
pub mod text;
