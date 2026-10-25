//! Export PDF du planning (ADR-0016 D8).
//!
//! Couche du rendu : ne dépend ni d'egui ni de `planrs-automation` (règle 9).
//! Écrit avec krilla 0.8.2 sans façonnage : chaque glyphe est placé à
//! l'avance de la table de largeurs d'Inter (H-R11), pour que coupures et
//! points de suspension soient identiques au SVG. Sortie déterministe (H-R13).
#![forbid(unsafe_code)]

mod smoke;

pub use smoke::smoke_page;

/// Erreur de l'export PDF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdfError {
    /// Police illisible (fichier tronqué ou non OpenType).
    FontInvalid,
    /// Dimension de page ou de forme refusée par krilla (non finie, nulle).
    Geometry,
    /// Échec de l'écriture du document par krilla.
    Write(String),
    /// Fonction posée par le contrat (L0), livrée par un lot ultérieur.
    NotYetAvailable,
}

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FontInvalid => f.write_str("police illisible"),
            Self::Geometry => f.write_str("géométrie refusée"),
            Self::Write(e) => write!(f, "écriture du PDF : {e}"),
            Self::NotYetAvailable => f.write_str("export PDF pas encore disponible"),
        }
    }
}

impl std::error::Error for PdfError {}
