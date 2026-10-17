//! Serveur MCP de planrs (SDK officiel `rmcp`, transport stdio).
//!
//! Il expose les commandes de `planrs-automation` à un agent IA. Il ne
//! calcule jamais une date lui-même et ne renvoie jamais de résultat estimé :
//! tout vient du moteur (règle 3). Au démarrage il se relie à l'application
//! ouverte si elle existe, sinon il travaille seul (ADR-0002).
//!
//! État : squelette du jalon J1, pas encore d'outils MCP.
