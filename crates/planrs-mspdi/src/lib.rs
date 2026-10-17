//! Import et export du format MSPDI XML (Microsoft Project Data Interchange).
//!
//! MSPDI est le format natif de planrs en V1 : il garantit l'échange avec
//! MS Project. L'implémentation suit le schéma XML public de Microsoft
//! (clean-room, règle 1) avec `quick-xml` et `serde`. Les flottants n'y
//! apparaissent qu'aux frontières, avec conversion explicite vers les entiers
//! du modèle (ADR-0005).
//!
//! État : squelette du jalon J1, pas encore de lecture ni d'écriture.
