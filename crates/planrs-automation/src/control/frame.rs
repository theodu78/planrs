//! Trames JSON par lignes du canal (≤ 8 Mio par ligne ; lot L10).
//!
//! Une requête [`Frame`] ou une réponse [`Reply`] tient sur une ligne
//! terminée par `\n` (un `\r` final est toléré en lecture). Une ligne plus
//! longue que [`MAX_LINE_BYTES`] est refusée sans être lue en entier : le
//! lecteur [`read_line_bounded`] ne garde jamais plus que la borne en mémoire.

use std::io::{self, BufRead};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Version du protocole.
pub const PROTOCOL_VERSION: u32 = 1;

/// Taille maximale d'une ligne, sans le `\n` (8 Mio).
pub const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;

/// Codes d'erreur stables du canal (champ `code` de [`WireError`]).
pub mod codes {
    /// Ligne plus longue que [`super::MAX_LINE_BYTES`].
    pub const LINE_TOO_LONG: &str = "control.line_too_long";
    /// Trame illisible (JSON invalide, champ inconnu ou manquant).
    pub const INVALID_FRAME: &str = "control.invalid_frame";
    /// Version de protocole non prise en charge.
    pub const UNSUPPORTED_VERSION: &str = "control.unsupported_version";
    /// Jeton absent ou faux dans `hello`.
    pub const BAD_TOKEN: &str = "control.bad_token";
    /// Requête avant un `hello` accepté.
    pub const UNAUTHENTICATED: &str = "control.unauthenticated";
    /// Méthode ou lecture non prise en charge (réservée à J7).
    pub const UNSUPPORTED: &str = "control.unsupported";
    /// Erreur interne de l'application (sérialisation).
    pub const INTERNAL: &str = "control.internal";
}

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
    /// Paramètres (`null` si absents).
    #[serde(default)]
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

impl Method {
    /// Toutes les méthodes, dans l'ordre de déclaration.
    pub const ALL: [Self; 8] = [
        Self::Hello,
        Self::Query,
        Self::Execute,
        Self::Undo,
        Self::Redo,
        Self::History,
        Self::UiState,
        Self::Screenshot,
    ];

    /// Nom sur le fil (`hello`, `ui_state`…).
    #[must_use]
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::Hello => "hello",
            Self::Query => "query",
            Self::Execute => "execute",
            Self::Undo => "undo",
            Self::Redo => "redo",
            Self::History => "history",
            Self::UiState => "ui_state",
            Self::Screenshot => "screenshot",
        }
    }

    /// Méthode servie par l'écouteur J4 (`hello`, `query`).
    #[must_use]
    pub const fn available_in_j4(self) -> bool {
        matches!(self, Self::Hello | Self::Query)
    }
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

impl Reply {
    /// Réponse d'erreur.
    #[must_use]
    pub fn error(id: u64, code: &str, message: impl Into<String>) -> Self {
        Self::Err {
            id,
            error: WireError {
                code: code.to_owned(),
                message: message.into(),
            },
        }
    }

    /// Identifiant de la requête à laquelle la réponse se rapporte.
    #[must_use]
    pub const fn id(&self) -> u64 {
        match self {
            Self::Ok { id, .. } | Self::Err { id, .. } | Self::PendingApproval { id } => *id,
        }
    }
}

/// Erreur transmise sur le canal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireError {
    /// Code stable ([`codes`]).
    pub code: String,
    /// Message.
    pub message: String,
}

/// Échec de lecture d'une ligne.
#[derive(Debug)]
pub enum LineError {
    /// Ligne plus longue que la borne ; le reste n'est pas lu.
    TooLong,
    /// Entrée/sortie (délai dépassé compris).
    Io(io::Error),
}

/// Lit une ligne d'au plus `max` octets (sans le `\n` ni un `\r` final).
///
/// `Ok(None)` en fin de flux sans octet lu ; une dernière ligne sans `\n`
/// est rendue telle quelle. La mémoire retenue ne dépasse jamais `max`.
///
/// # Errors
///
/// [`LineError::TooLong`] dès que la ligne dépasse `max` ;
/// [`LineError::Io`] sur une erreur du flux.
pub fn read_line_bounded<R: BufRead>(reader: &mut R, max: usize) -> Result<Option<Vec<u8>>, LineError> {
    let mut line = Vec::new();
    loop {
        let buf = match reader.fill_buf() {
            Ok(buf) => buf,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(LineError::Io(e)),
        };
        if buf.is_empty() {
            return Ok(if line.is_empty() {
                None
            } else {
                Some(trim_cr(line))
            });
        }
        let (chunk_len, found) = match buf.iter().position(|&b| b == b'\n') {
            Some(at) => (at, true),
            None => (buf.len(), false),
        };
        if line.len().saturating_add(chunk_len) > max {
            return Err(LineError::TooLong);
        }
        line.extend(buf.iter().take(chunk_len));
        let consumed = if found {
            chunk_len.saturating_add(1)
        } else {
            chunk_len
        };
        reader.consume(consumed);
        if found {
            return Ok(Some(trim_cr(line)));
        }
    }
}

fn trim_cr(mut line: Vec<u8>) -> Vec<u8> {
    if line.last() == Some(&b'\r') {
        line.pop();
    }
    line
}

/// Ligne JSON d'une valeur, terminée par `\n`.
///
/// # Errors
///
/// Erreur de sérialisation de `serde_json`.
pub fn to_line<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    let mut out = serde_json::to_vec(value)?;
    out.push(b'\n');
    Ok(out)
}
