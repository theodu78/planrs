//! Session « headless » de planrs : projet ouvert, historique et registre de
//! commandes.
//!
//! Toute action (menu, raccourci, CLI, MCP, canal de contrôle) passe par le
//! même registre de commandes, identifiées par un id stable (`task.add`,
//! `link.add`…). Cette crate ne dépend d'aucune interface graphique
//! (règle 9, vérifiée par `cargo xtask layers`).
//!
//! État : squelette du jalon J1, pas encore de registre.
