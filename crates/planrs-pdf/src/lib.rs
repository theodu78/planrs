//! Export PDF du planning (ADR-0016 D8).
//!
//! Couche du rendu : ne dépend ni d'egui ni de `planrs-automation` (règle 9).
//! Écrit avec krilla 0.8.2 sans façonnage : chaque glyphe est placé à
//! l'avance de la table de largeurs d'Inter (H-R11), pour que coupures et
//! points de suspension soient identiques au SVG. Sortie déterministe (H-R13).
#![forbid(unsafe_code)]

mod smoke;

use planrs_core::Moment;
use planrs_render::paginate::Pages;
use planrs_render::{Lang, RenderReport};

pub use smoke::smoke_page;

/// Inter Regular embarquée (`assets/fonts`, OFL-1.1, `NOTICE`).
pub static INTER_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
/// Inter `SemiBold` embarquée.
pub static INTER_SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf");

/// Métadonnées du document (H-R13 : date et identifiant injectés, jamais
/// d'horloge).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PdfMeta {
    /// Titre.
    pub title: String,
    /// Date de création, fournie par l'appelant.
    pub created: Moment,
    /// Langue du document.
    pub lang: Lang,
}

/// Polices du PDF ; `fallback` : police de l'utilisateur pour les
/// caractères absents d'Inter (CJK, desktop seulement), sinon
/// `pdf.glyph_missing`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontSet {
    /// Texte courant.
    pub regular: &'static [u8],
    /// Texte appuyé.
    pub semibold: &'static [u8],
    /// Police de repli.
    pub fallback: Option<Vec<u8>>,
}

impl FontSet {
    /// Inter seule (Regular et `SemiBold` du dépôt).
    #[must_use]
    pub fn inter() -> Self {
        Self {
            regular: INTER_REGULAR,
            semibold: INTER_SEMIBOLD,
            fallback: None,
        }
    }
}

/// Écrit le PDF des pages. Squelette L0 (export en L7) :
/// [`PdfError::NotYetAvailable`].
///
/// # Errors
///
/// [`PdfError`].
pub fn pdf(pages: &Pages, fonts: &FontSet, meta: &PdfMeta) -> Result<(Vec<u8>, RenderReport), PdfError> {
    let _ = (pages, fonts, meta);
    Err(PdfError::NotYetAvailable)
}

/// Erreur de l'export PDF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdfError {
    /// Police illisible (fichier tronqué ou non OpenType).
    FontInvalid,
    /// Dimension de page ou de forme refusée par krilla (non finie, nulle).
    Geometry,
    /// Échec de l'écriture du document par krilla.
    Write(String),
    /// Plus de pages que `MAX_PAGES` (`pdf.too_many_pages`).
    TooManyPages(usize),
    /// Caractère absent des polices (`pdf.glyph_missing`).
    GlyphMissing(char),
    /// Logo illisible ou hors bornes (`pdf.logo_invalid`).
    LogoInvalid,
    /// Fonction posée par le contrat (L0), livrée par un lot ultérieur.
    NotYetAvailable,
}

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FontInvalid => f.write_str("police illisible"),
            Self::Geometry => f.write_str("géométrie refusée"),
            Self::Write(e) => write!(f, "écriture du PDF : {e}"),
            Self::TooManyPages(n) => write!(f, "{n} pages, 1 000 au plus"),
            Self::GlyphMissing(c) => write!(f, "caractère absent des polices : U+{:04X}", u32::from(*c)),
            Self::LogoInvalid => f.write_str("logo illisible"),
            Self::NotYetAvailable => f.write_str("export PDF pas encore disponible"),
        }
    }
}

impl std::error::Error for PdfError {}
