//! Journal de rejeu `--journal` : requête écrite (et synchronisée) avant
//! l'appel, résultat après (lot L8).

#![allow(
    dead_code,
    reason = "contrat J4 (L0) : écrit en L8 ; attribut à retirer en L8"
)]

use std::path::PathBuf;

/// Fichier de journal ouvert.
#[derive(Debug)]
pub(crate) struct JournalFile {
    path: PathBuf,
    seq: u64,
}
