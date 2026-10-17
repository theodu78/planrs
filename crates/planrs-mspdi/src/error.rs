//! Erreurs de lecture d'un document MSPDI et positions dans l'entrée.
//!
//! [`MspdiError`] n'est rendue que si le document est structurellement
//! inexploitable (vide, trop grand, mal formé, trop profond, autre racine) ;
//! tout le reste devient un diagnostic d'import (`crate::diag`).

use std::fmt;

/// Position dans l'entrée : octet (depuis 0), ligne et colonne (depuis 1,
/// colonne comptée en caractères). Calculée à la construction de l'erreur ou
/// du diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location {
    /// Décalage en octets depuis le début de l'entrée (après le BOM éventuel).
    pub byte: u64,
    /// Ligne, depuis 1.
    pub line: u32,
    /// Colonne en caractères, depuis 1.
    pub column: u32,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ligne {}, colonne {} (octet {})",
            self.line, self.column, self.byte
        )
    }
}

/// Erreur de lecture : document inexploitable.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MspdiError {
    /// Entrée vide.
    #[error("document vide")]
    Empty,
    /// Entrée plus grande que la limite.
    #[error("document trop grand : {size} octets (limite {max})")]
    TooLarge {
        /// Taille de l'entrée.
        size: u64,
        /// Limite.
        max: u64,
    },
    /// Encodage UTF-16 (BOM) : non pris en charge, MS Project écrit en UTF-8.
    #[error("document en UTF-16 : seul l'UTF-8 est pris en charge")]
    Utf16NotSupported,
    /// Octets qui ne sont pas de l'UTF-8.
    #[error("UTF-8 invalide à la {at}")]
    InvalidUtf8 {
        /// Premier octet invalide.
        at: Location,
    },
    /// XML mal formé.
    #[error("XML mal formé à la {at} : {message}")]
    Xml {
        /// Position de l'erreur.
        at: Location,
        /// Message de l'analyseur.
        message: String,
    },
    /// Fin de l'entrée avec des éléments encore ouverts (fichier tronqué).
    #[error("fin inattendue du document à la {at} (éléments ouverts : {})", open.join("/"))]
    UnexpectedEof {
        /// Fin de l'entrée.
        at: Location,
        /// Chemin des éléments ouverts, de la racine au plus profond.
        open: Vec<String>,
    },
    /// Déclaration DOCTYPE refusée (entités, expansion).
    #[error("déclaration DOCTYPE refusée à la {at}")]
    DoctypeNotAllowed {
        /// Position de la déclaration.
        at: Location,
    },
    /// Imbrication plus profonde que la limite.
    #[error("imbrication trop profonde à la {at} (limite {max} niveaux)")]
    TooDeep {
        /// Élément qui dépasse la limite.
        at: Location,
        /// Limite.
        max: u16,
    },
    /// Plus d'éléments que la limite.
    #[error("trop d'éléments (limite {max})")]
    TooManyElements {
        /// Limite.
        max: u64,
    },
    /// Texte d'un élément plus long que la limite.
    #[error("texte trop long à la {at} (limite {max} octets)")]
    TextTooLong {
        /// Texte en cause.
        at: Location,
        /// Limite.
        max: usize,
    },
    /// La racine n'est pas `Project` : ce n'est pas un document MSPDI.
    #[error("pas un document MSPDI : racine « {root} » au lieu de « Project »")]
    NotMspdi {
        /// Nom local de la racine trouvée.
        root: String,
    },
}

impl MspdiError {
    /// Code stable (`mspdi.error.too_deep`…).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Empty => "mspdi.error.empty",
            Self::TooLarge { .. } => "mspdi.error.too_large",
            Self::Utf16NotSupported => "mspdi.error.utf16_not_supported",
            Self::InvalidUtf8 { .. } => "mspdi.error.invalid_utf8",
            Self::Xml { .. } => "mspdi.error.xml",
            Self::UnexpectedEof { .. } => "mspdi.error.unexpected_eof",
            Self::DoctypeNotAllowed { .. } => "mspdi.error.doctype_not_allowed",
            Self::TooDeep { .. } => "mspdi.error.too_deep",
            Self::TooManyElements { .. } => "mspdi.error.too_many_elements",
            Self::TextTooLong { .. } => "mspdi.error.text_too_long",
            Self::NotMspdi { .. } => "mspdi.error.not_mspdi",
        }
    }
}

/// Index des débuts de ligne d'un texte : octet → ligne et colonne.
pub(crate) struct LineIndex<'a> {
    text: &'a str,
    /// Octet de début de chaque ligne ; le premier vaut 0.
    starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    /// Un parcours de l'entrée.
    pub(crate) fn new(text: &'a str) -> Self {
        let mut starts = vec![0];
        starts.extend(
            text.bytes()
                .enumerate()
                .filter(|(_, b)| *b == b'\n')
                .map(|(i, _)| i.saturating_add(1)),
        );
        Self { text, starts }
    }

    /// Position de l'octet `byte` (borné à la longueur du texte).
    pub(crate) fn locate(&self, byte: u64) -> Location {
        let len = self.text.len();
        let offset = usize::try_from(byte).map_or(len, |b| b.min(len));
        let line_index = self.starts.partition_point(|s| *s <= offset).saturating_sub(1);
        let start = self.starts.get(line_index).copied().unwrap_or(0);
        let column = self
            .text
            .get(start..offset)
            .map_or_else(|| offset.saturating_sub(start), |s| s.chars().count());
        Location {
            byte: u64::try_from(offset).unwrap_or(u64::MAX),
            line: u32::try_from(line_index.saturating_add(1)).unwrap_or(u32::MAX),
            column: u32::try_from(column.saturating_add(1)).unwrap_or(u32::MAX),
        }
    }
}

/// Position d'un octet invalide dans une entrée qui n'est pas de l'UTF-8 :
/// le préfixe `bytes[..valid_up_to]` est valide.
pub(crate) fn locate_in_bytes(bytes: &[u8], valid_up_to: usize) -> Location {
    let prefix = bytes
        .get(..valid_up_to)
        .and_then(|p| std::str::from_utf8(p).ok())
        .unwrap_or("");
    LineIndex::new(prefix).locate(u64::try_from(prefix.len()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::{LineIndex, Location, MspdiError, locate_in_bytes};

    #[test]
    fn line_index() {
        let text = "ab\ncdé\n\nf";
        let idx = LineIndex::new(text);
        assert_eq!(
            idx.locate(0),
            Location {
                byte: 0,
                line: 1,
                column: 1
            }
        );
        assert_eq!(idx.locate(2).column, 3);
        assert_eq!(
            idx.locate(3),
            Location {
                byte: 3,
                line: 2,
                column: 1
            }
        );
        // « é » compte pour une colonne : l'octet 7 (après « cdé ») est en colonne 4.
        assert_eq!(idx.locate(7).column, 4);
        assert_eq!(idx.locate(8).line, 3);
        assert_eq!(idx.locate(9).line, 4);
        // Au-delà de la fin : bornée.
        assert_eq!(idx.locate(1_000).byte, 10);
        // Au milieu d'un caractère : colonne en octets, sans panique.
        assert_eq!(idx.locate(6).line, 2);
        assert_eq!(LineIndex::new("").locate(5).byte, 0);
    }

    #[test]
    fn invalid_utf8_location() {
        let bytes = b"<a>\n  \xff</a>";
        let loc = locate_in_bytes(bytes, 6);
        assert_eq!(
            loc,
            Location {
                byte: 6,
                line: 2,
                column: 3
            }
        );
    }

    #[test]
    fn codes_and_messages() {
        let e = MspdiError::UnexpectedEof {
            at: Location {
                byte: 10,
                line: 2,
                column: 3,
            },
            open: vec!["Project".to_owned(), "Tasks".to_owned()],
        };
        assert_eq!(e.code(), "mspdi.error.unexpected_eof");
        assert!(e.to_string().contains("Project/Tasks"), "{e}");
        assert_eq!(MspdiError::Empty.code(), "mspdi.error.empty");
    }
}
