//! Outils communs aux harnais de tests transverses de planrs.
//!
//! Ce paquet (`planrs-tests`, non publié) regroupe trois cibles de test :
//! - `rules` (`tests/rules/`) : un test par règle de calcul `R-xxx` ;
//! - `oracle` (`tests/oracle/`) : comparaison avec des fichiers recalculés par
//!   MS Project, quand une licence sera disponible (ADR-0004) ;
//! - `corpus` (`tests/corpus/`) : fichiers MSPDI publics, recalculés et
//!   comparés aux dates enregistrées par MS Project (harnais complet en J2).

#![allow(
    clippy::panic,
    reason = "paquet de test : une aide qui reçoit une donnée de test invalide doit faire échouer le test"
)]

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

/// Instant depuis `"2026-10-19T08:00"` ; panique sur une écriture invalide
/// (aide de test).
///
/// # Panics
///
/// Si le texte n'est pas un instant valide du domaine.
#[must_use]
pub fn moment(s: &str) -> planrs_core::Moment {
    match s.parse() {
        Ok(m) => m,
        Err(e) => panic!("instant de test invalide « {s} » : {e}"),
    }
}

/// Jour depuis `"2026-12-25"` ; panique sur une écriture invalide (aide de test).
///
/// # Panics
///
/// Si le texte n'est pas un jour valide du domaine.
#[must_use]
pub fn day(s: &str) -> planrs_core::Day {
    match s.parse() {
        Ok(d) => d,
        Err(e) => panic!("jour de test invalide « {s} » : {e}"),
    }
}
