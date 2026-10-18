//! Trames JSON par lignes du canal (≤ 8 Mio par ligne ; lot L10).

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Requête.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    /// Version du protocole (1).
    pub v: u32,
    /// Identifiant de la requête.
    pub id: u64,
    /// Méthode.
    pub method: Method,
    /// Paramètres.
    pub params: Value,
}

/// Méthode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// Présentation avec le jeton.
    Hello,
    /// Lecture.
    Query,
    /// Lot de commandes (J7).
    Execute,
    /// Annuler (J7).
    Undo,
    /// Rétablir (J7).
    Redo,
    /// Historique (J7).
    History,
    /// État de l'interface (réservé).
    UiState,
    /// Capture (réservé).
    Screenshot,
}

/// Réponse.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reply {
    /// Succès.
    Ok {
        /// Requête.
        id: u64,
        /// Résultat.
        result: Value,
    },
    /// Échec.
    Err {
        /// Requête.
        id: u64,
        /// Erreur.
        error: WireError,
    },
    /// En attente d'approbation (réservé J7).
    PendingApproval {
        /// Requête.
        id: u64,
    },
}

/// Erreur transmise sur le canal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireError {
    /// Code stable.
    pub code: String,
    /// Message.
    pub message: String,
}
