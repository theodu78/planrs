//! Pagination de la scène (ADR-0016 D8, H-UI-48 à H-UI-51, P54).
//!
//! Contrat posé par le lot L0, implémenté en L7 : bandes de pages sans ligne
//! coupée, récapitulative jamais seule en bas de page, parent rappelé
//! « (suite) », en-têtes de colonnes et échelle répétés, « N pages en
//! largeur », au plus [`MAX_PAGES`] pages (`pdf.too_many_pages`).

use std::ops::Range;

use crate::geometry::{BarTextSource, Scene};
use crate::style::PageStyle;

/// Plafond du nombre de pages d'un export (règle 10 : entrée bornée).
pub const MAX_PAGES: usize = 1_000;

/// Une page : bande de lignes et tranche d'abscisses de la scène.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    /// Rang en largeur (0 = première colonne de pages).
    pub col: u16,
    /// Rang en hauteur.
    pub row: u16,
    /// Lignes de la scène portées par la page.
    pub rows: Range<usize>,
    /// Tranche d'abscisses (1/16 px) de la scène.
    pub x16: (i64, i64),
    /// Ligne rappelée « (suite) » en haut de page.
    pub continued_parent: Option<usize>,
    /// Zones d'en-tête, jetons remplacés (gauche, centre, droite).
    pub header: [String; 3],
    /// Zones de pied de page, jetons remplacés.
    pub footer: [String; 3],
}

/// Pages d'un export, dans l'ordre de lecture.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pages {
    /// Pages.
    pub pages: Vec<Page>,
    /// Largeur et hauteur du papier en points (1/72 pouce), orientation
    /// appliquée.
    pub paper_pt: (u32, u32),
}

/// Erreur de pagination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageError {
    /// Plus de [`MAX_PAGES`] pages nécessaires.
    TooManyPages {
        /// Pages nécessaires (au moins `MAX_PAGES + 1`).
        needed: usize,
    },
    /// Pagination livrée par le lot L7.
    NotYetAvailable,
}

impl std::fmt::Display for PageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyPages { needed } => {
                write!(f, "{needed} pages nécessaires, {MAX_PAGES} au plus")
            }
            Self::NotYetAvailable => f.write_str("pagination pas encore disponible"),
        }
    }
}

impl std::error::Error for PageError {}

/// Découpe la scène en pages. Squelette L0 : [`PageError::NotYetAvailable`].
///
/// # Errors
///
/// [`PageError`].
pub fn paginate(scene: &Scene, page: &PageStyle, src: &dyn BarTextSource) -> Result<Pages, PageError> {
    let _ = (scene, page, src);
    Err(PageError::NotYetAvailable)
}
