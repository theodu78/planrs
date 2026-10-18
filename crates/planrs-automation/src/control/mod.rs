//! Canal de contrôle de l'application (ADR-0002) : protocole, rendez-vous,
//! écouteur `hello` (hors wasm ; lot L10). En J4, le canal garde le fichier
//! (pas de double écrivain) sans relayer les commandes.

mod frame;
mod rendezvous;

use std::sync::{Arc, Mutex};

pub use frame::{Frame, Method, Reply, WireError};
pub use rendezvous::{Rendezvous, RendezvousGuard, find_live_for, publish, state_dir};

use crate::view::SummaryView;

/// Erreur du canal.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ControlError {
    /// Pas de dossier d'état sur cette plateforme.
    #[error("dossier d'état introuvable")]
    NoStateDir,
    /// Entrée/sortie.
    #[error("canal de contrôle : {0}")]
    Io(String),
    /// Non pris en charge (plateforme, ou squelette L0).
    #[error("canal de contrôle non pris en charge")]
    Unsupported,
}

/// Écouteur en cours ; arrêté à sa destruction (L10).
#[derive(Debug)]
pub struct ListenerHandle {
    _private: (),
}

/// Lance l'écouteur `hello`/`query` dans un fil dédié. Squelette L0.
///
/// # Errors
///
/// [`ControlError`] ; en L0, toujours [`ControlError::Unsupported`].
pub fn spawn_hello_listener(
    token: String,
    summary: Arc<Mutex<Option<SummaryView>>>,
) -> Result<ListenerHandle, ControlError> {
    drop((token, summary));
    Err(ControlError::Unsupported)
}
