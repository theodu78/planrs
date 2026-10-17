//! Outils communs aux harnais de tests transverses de planrs.
//!
//! Ce paquet (`planrs-tests`, non publié) regroupe trois cibles de test :
//! - `rules` (`tests/rules/`) : un test par règle de calcul `R-xxx` ;
//! - `oracle` (`tests/oracle/`) : comparaison avec des fichiers recalculés par
//!   MS Project, quand une licence sera disponible (ADR-0004) ;
//! - `corpus` (`tests/corpus/`) : fichiers MSPDI publics, recalculés et
//!   comparés aux dates enregistrées par MS Project (harnais complet en J2).

use std::path::{Path, PathBuf};

/// Dossier `tests/` du dépôt, quel que soit le répertoire courant.
#[must_use]
pub fn tests_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Chemin d'un fichier ou dossier relatif à `tests/`.
#[must_use]
pub fn tests_path(relative: &str) -> PathBuf {
    tests_dir().join(relative)
}
