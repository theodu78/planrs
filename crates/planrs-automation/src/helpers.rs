//! Raccourcis d'édition de l'interface (R-252, R-253, R-255 ; ADR-0016 D4).
//!
//! Chaque raccourci rend un lot ordinaire (`progress.update`, `task.update`)
//! appliqué par le registre : une entrée d'annulation, `dry_run` gratuit ;
//! `None` quand il n'y a rien à faire. Contrat posé par le lot L0,
//! implémenté en L2 : jusque-là [`HelperError::NotYetAvailable`].

use planrs_core::TaskUid;

use crate::command::Batch;
use crate::session::Session;

/// Pas de « marquer n % » (boutons 0 à 100 %, H-UI-35).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PercentStep {
    /// 0 %.
    P0,
    /// 25 %.
    P25,
    /// 50 %.
    P50,
    /// 75 %.
    P75,
    /// 100 %.
    P100,
}

impl PercentStep {
    /// % achevé en centièmes (10 000 = 100 %).
    #[must_use]
    pub const fn hundredths(self) -> u16 {
        match self {
            Self::P0 => 0,
            Self::P25 => 2_500,
            Self::P50 => 5_000,
            Self::P75 => 7_500,
            Self::P100 => 10_000,
        }
    }
}

/// Refus d'un raccourci.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum HelperError {
    /// Tâche inconnue.
    #[error("tâche inconnue : {0:?}")]
    UnknownTask(TaskUid),
    /// Raccourci livré par le lot L2.
    #[error("raccourci pas encore disponible")]
    NotYetAvailable,
}

/// R-252 : marque n % les feuilles de la sélection. Squelette L0.
///
/// # Errors
///
/// [`HelperError`].
pub fn mark_percent(s: &Session, uids: &[TaskUid], p: PercentStep) -> Result<Option<Batch>, HelperError> {
    let _ = (s, uids, p);
    Err(HelperError::NotYetAvailable)
}

/// R-253 : bascule auto ↔ manuel. Squelette L0.
///
/// # Errors
///
/// [`HelperError`].
pub fn toggle_mode(s: &Session, uids: &[TaskUid]) -> Result<Option<Batch>, HelperError> {
    let _ = (s, uids);
    Err(HelperError::NotYetAvailable)
}

/// R-255 : bascule actif / inactif. Squelette L0.
///
/// # Errors
///
/// [`HelperError`].
pub fn toggle_active(s: &Session, uids: &[TaskUid]) -> Result<Option<Batch>, HelperError> {
    let _ = (s, uids);
    Err(HelperError::NotYetAvailable)
}
