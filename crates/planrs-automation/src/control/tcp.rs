//! Transport TCP en boucle locale du canal de contrôle (C-CTL-11 à
//! C-CTL-14, ADR-0016 D12) : écouteur lié à `127.0.0.1:0`, pair hors boucle
//! locale fermé sans lecture, première ligne invalide ⇒ connexion fermée.
//! Contrat posé par le lot L0, implémenté en L8 : jusque-là aucun écouteur
//! (`ControlError::Unsupported`) et tout pair refusé.

use std::net::{SocketAddr, TcpListener};

use super::ControlError;

/// Écouteur lié à `127.0.0.1:0` (port choisi par le système). Squelette L0.
///
/// # Errors
///
/// [`ControlError::Unsupported`] jusqu'à L8.
pub fn bind_loopback() -> Result<TcpListener, ControlError> {
    Err(ControlError::Unsupported)
}

/// Vrai si le pair peut être lu (C-CTL-12 : boucle locale seulement).
/// Squelette L0 : tout pair est refusé.
#[must_use]
pub fn accept_peer(peer: &SocketAddr) -> bool {
    let _ = peer;
    false
}
