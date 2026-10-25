//! Aperçu d'un CSV avant import (H-UI-40, ADR-0016 ; lot L3).
//!
//! Lit les 20 premières lignes de données avec les options de l'import
//! (séparateur, encodage, langue modifiables) et dit, colonne par colonne,
//! ce que l'import en fera. Contrat posé par le lot L0 : jusqu'à L3,
//! [`CsvError::NotYetAvailable`].

use super::{CsvColumn, CsvDiagnostic, CsvError, CsvImportOptions, Detected};

/// Lignes de données au plus dans l'aperçu.
pub const PREVIEW_ROWS: usize = 20;

/// Sort d'une colonne d'en-tête.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recognized {
    /// Colonne lue.
    Column(CsvColumn),
    /// Colonne connue, non prise en charge.
    Unsupported,
    /// En-tête inconnu : ignorée.
    Unknown,
    /// Colonne en double : ignorée (la première gagne).
    Duplicate,
}

/// Aperçu d'un CSV.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CsvPreview {
    /// Séparateur, langue et encodage retenus.
    pub detected: Detected,
    /// En-têtes tels qu'écrits et leur sort.
    pub columns: Vec<(String, Recognized)>,
    /// Au plus [`PREVIEW_ROWS`] lignes de données, cellules brutes.
    pub rows: Vec<Vec<String>>,
    /// Diagnostics des lignes montrées.
    pub diagnostics: Vec<CsvDiagnostic>,
}

/// Aperçu de `bytes`. Squelette L0.
///
/// # Errors
///
/// [`CsvError`] (refus du fichier entier, comme l'import).
pub fn preview(bytes: &[u8], opts: &CsvImportOptions) -> Result<CsvPreview, CsvError> {
    let _ = (bytes, opts);
    Err(CsvError::NotYetAvailable)
}
