//! Codes de sortie (C-MCP-08).

use std::process::ExitCode;

/// Succès.
#[allow(dead_code, reason = "contrat J4 (L0) : utilisé par les sous-commandes (L9)")]
pub const SUCCESS: u8 = 0;
/// Commande refusée (erreur métier).
#[allow(dead_code, reason = "contrat J4 (L0) : utilisé par les sous-commandes (L9)")]
pub const REFUSED: u8 = 1;
/// Usage (arguments, commande indisponible).
pub const USAGE: u8 = 2;
/// Fichier (lecture, écriture, MSPDI illisible).
#[allow(dead_code, reason = "contrat J4 (L0) : utilisé par les sous-commandes (L9)")]
pub const FILE: u8 = 3;

/// Sous-commande pas encore écrite : message sur la sortie d'erreur, code
/// d'usage.
pub fn not_yet(name: &str) -> ExitCode {
    eprintln!("planrs {name} : pas encore disponible");
    ExitCode::from(USAGE)
}
