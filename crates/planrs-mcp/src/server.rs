//! `PlanrsServer` et son `ServerHandler` (lot L8).

#![allow(
    dead_code,
    reason = "contrat J4 (L0) : champs lus par le ServerHandler (L8) ; attribut à retirer en L8"
)]

use std::sync::{Arc, Mutex};

use planrs_automation::Workspace;

use crate::journal_file::JournalFile;

/// Serveur MCP : un espace de travail partagé, un journal facultatif.
#[derive(Clone, Debug)]
pub struct PlanrsServer {
    ws: Arc<Mutex<Workspace>>,
    journal: Option<Arc<Mutex<JournalFile>>>,
}
