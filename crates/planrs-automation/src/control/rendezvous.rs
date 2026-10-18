//! Rendez-vous : fichier 0600 dans un dossier 0700 qui annonce une
//! application vivante (lot L10).

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::ControlError;

/// Contenu du fichier de rendez-vous.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rendezvous {
    /// Version (1).
    pub v: u32,
    /// Processus de l'application.
    pub pid: u32,
    /// Adresse de l'écouteur (socket Unix).
    pub endpoint: String,
    /// Jeton (64 chiffres hexadécimaux).
    pub token: String,
    /// Chemin canonique du projet ouvert.
    pub project: Option<String>,
}

impl fmt::Debug for Rendezvous {
    // Le jeton n'apparaît jamais dans une trace.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rendezvous")
            .field("v", &self.v)
            .field("pid", &self.pid)
            .field("endpoint", &self.endpoint)
            .field("token", &"…")
            .field("project", &self.project)
            .finish()
    }
}

/// Fichier publié, supprimé à la destruction.
#[derive(Debug)]
pub struct RendezvousGuard {
    path: PathBuf,
}

impl Drop for RendezvousGuard {
    fn drop(&mut self) {
        // Échec ignoré : un rendez-vous périmé est nettoyé par
        // `find_live_for`.
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Dossier des rendez-vous de la plateforme. Squelette L0 : `None`.
#[must_use]
pub fn state_dir() -> Option<PathBuf> {
    None
}

/// Publie un rendez-vous. Squelette L0.
///
/// # Errors
///
/// [`ControlError`] ; en L0, toujours [`ControlError::Unsupported`].
pub fn publish(r: &Rendezvous) -> Result<RendezvousGuard, ControlError> {
    let _ = r;
    Err(ControlError::Unsupported)
}

/// Rendez-vous vivant d'une application qui tient `path`. Squelette L0 :
/// `None`.
#[must_use]
pub fn find_live_for(path: &Path) -> Option<Rendezvous> {
    let _ = path;
    None
}
