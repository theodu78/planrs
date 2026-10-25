//! Volet latéral (ADR-0016 D10) : onglets « Opérations IA » (H-UI-41,
//! table avant/après, Accepter / Refuser, historique de session) et
//! « Pourquoi cette date ? » (H-UI-58). Contrat posé par le lot L0,
//! implémenté en L12.

/// Entrées au plus dans l'historique du panneau (règle 10 de J8).
pub const MAX_HISTORY: usize = 50;
/// Lignes au plus dans un diff affiché (règle 10 de J8).
pub const MAX_DIFF_ROWS: usize = 200;

/// Onglet du volet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SideTab {
    /// Opérations IA.
    #[default]
    AiOperations,
    /// Pourquoi cette date ?
    WhyThisDate,
}
