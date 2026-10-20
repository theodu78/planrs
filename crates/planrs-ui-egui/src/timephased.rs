//! Données d'utilisation par période pour la vue « Utilisation des
//! ressources » (J7, ADR-0015 D7 ; lot L11) : valeurs du moteur
//! (`planrs_core::resource::usage_with`, `usage_by_assignment`) par blocs
//! de [`CHUNK_PERIODS`] périodes, pour les seules ressources visibles,
//! gardées en cache par génération ([`MAX_CHUNKS`] blocs au plus).
//!
//! Contrat J7 (lot L0) : bornes seulement.

/// Périodes d'un bloc.
pub const CHUNK_PERIODS: usize = 64;

/// Blocs gardés en cache au plus (les plus anciens sortent).
pub const MAX_CHUNKS: usize = 8;
