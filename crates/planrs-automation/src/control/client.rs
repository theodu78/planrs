//! Client du relais côté `planrs mcp` (J7, ADR-0015 D9 ; contrat du lot
//! L0, protocole au lot L2) : se rattache à l'application qui tient le
//! fichier et lui relaie les appels d'outils.
//!
//! Un appel d'écriture reçoit d'abord `pending_approval`, puis la réponse
//! finale sur la même connexion, au plus `approval` plus une marge ; une
//! erreur d'E/S est `control.app_gone` (C-CTL-06).

use std::time::Duration;

use serde_json::Value;

use super::{ControlError, Rendezvous};
use crate::error::codes;
use crate::workspace::ToolError;

/// Connexion de `planrs mcp` à l'application rattachée.
#[derive(Debug)]
pub struct RelayClient {
    /// Identifiant de la prochaine requête.
    next_id: u64,
}

impl RelayClient {
    /// Se connecte au rendez-vous `r` (`hello` avec son jeton), en
    /// `timeout` au plus.
    ///
    /// # Errors
    ///
    /// [`ControlError`] ; squelette J7 L0 : [`ControlError::Unsupported`].
    pub fn connect(r: &Rendezvous, timeout: Duration) -> Result<Self, ControlError> {
        let _ = (r, timeout);
        Err(ControlError::Unsupported)
    }

    /// Relaie l'outil `tool` (id du registre) ; une écriture attend la
    /// validation de l'utilisateur `approval` au plus.
    ///
    /// # Errors
    ///
    /// [`ToolError`] : celle de l'application, ou `control.*` ;
    /// squelette J7 L0 : `control.app_gone`.
    pub fn call(&mut self, tool: &str, params: Value, approval: Duration) -> Result<Value, ToolError> {
        let _ = (params, approval);
        self.next_id = self.next_id.saturating_add(1);
        Err(ToolError::simple(
            codes::CONTROL_APP_GONE,
            format!("relais non disponible : {tool}"),
        ))
    }
}
