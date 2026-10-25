//! Barre de recherche (H-UI-33, Ctrl/Cmd+F ; ADR-0016 D2). Une recherche
//! est une vue : rien n'est écrit (P50). Contrat posé par le lot L0,
//! implémenté en L9.

use crate::row_view::ModelRow;

/// Longueur maximale d'une requête, en caractères (règle 10 de J8).
pub const MAX_QUERY_CHARS: usize = 256;

/// Portée de la recherche.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchScope {
    /// Nom des tâches.
    #[default]
    Name,
    /// Colonnes visibles.
    VisibleColumns,
}

/// État de la barre de recherche.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchState {
    /// Requête (≤ [`MAX_QUERY_CHARS`]).
    pub query: String,
    /// Portée.
    pub scope: SearchScope,
    /// Correspondances, dans l'ordre du modèle (masquées par le filtre
    /// comprises).
    pub hits: Vec<ModelRow>,
    /// Correspondance courante.
    pub cursor: usize,
}
