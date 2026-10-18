//! Fichiers : lecture bornée, écriture atomique, confinement (hors wasm ;
//! lot L6).

use std::path::{Path, PathBuf};

use crate::error::codes;

/// Erreur de fichier.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FileError {
    /// Lecture impossible.
    #[error("lecture impossible : {path} ({message})")]
    Read {
        /// Chemin.
        path: String,
        /// Cause.
        message: String,
    },
    /// Écriture impossible.
    #[error("écriture impossible : {path} ({message})")]
    Write {
        /// Chemin.
        path: String,
        /// Cause.
        message: String,
    },
    /// Fichier trop gros.
    #[error("fichier trop gros : {path} (au plus {max} octets)")]
    TooLarge {
        /// Chemin.
        path: String,
        /// Taille maximale.
        max: u64,
    },
    /// Chemin hors de la racine autorisée.
    #[error("chemin hors de la racine autorisée : {path}")]
    OutsideRoot {
        /// Chemin.
        path: String,
    },
}

impl FileError {
    /// Code stable.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Read { .. } => codes::FILE_READ,
            Self::Write { .. } => codes::FILE_WRITE,
            Self::TooLarge { .. } => codes::FILE_TOO_LARGE,
            Self::OutsideRoot { .. } => codes::FILE_OUTSIDE_ROOT,
        }
    }
}

const NOT_YET: &str = "pas encore disponible (J4, L6)";

/// Lit au plus `max` octets. Squelette L0.
///
/// # Errors
///
/// [`FileError`] ; en L0, toujours [`FileError::Read`].
pub fn read_path(p: &Path, max: u64) -> Result<Vec<u8>, FileError> {
    let _ = max;
    Err(FileError::Read {
        path: p.display().to_string(),
        message: NOT_YET.to_owned(),
    })
}

/// Écrit atomiquement (fichier temporaire du même dossier, fsync, rename).
/// Squelette L0.
///
/// # Errors
///
/// [`FileError`] ; en L0, toujours [`FileError::Write`].
pub fn write_atomic(p: &Path, bytes: &[u8]) -> Result<(), FileError> {
    let _ = bytes;
    Err(FileError::Write {
        path: p.display().to_string(),
        message: NOT_YET.to_owned(),
    })
}

/// Chemin canonique de `p`, refusé hors de `root`. Squelette L0.
///
/// # Errors
///
/// [`FileError`] ; en L0, toujours [`FileError::OutsideRoot`].
pub fn confine(root: &Path, p: &Path) -> Result<PathBuf, FileError> {
    let _ = root;
    Err(FileError::OutsideRoot {
        path: p.display().to_string(),
    })
}
