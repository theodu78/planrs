//! Lecteur borné : refuse une ligne d'entrée plus longue que la borne
//! (`io::ErrorKind::InvalidData`), le transport rmcp ferme alors proprement
//! (C-MCP-06, lot L8).

#![allow(
    dead_code,
    reason = "contrat J4 (L0) : AsyncRead écrit en L8 ; attribut à retirer en L8"
)]

/// Lecteur qui compte les octets depuis le dernier `\n`.
#[derive(Debug)]
pub(crate) struct BoundedLines<R> {
    inner: R,
    max_line_bytes: usize,
    current: usize,
}
