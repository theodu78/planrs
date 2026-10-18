//! Allocation des UID de tâche (R-151, lot L2).

#![allow(
    dead_code,
    reason = "contrat J4 (L0) : appelé par Session::apply (L2) ; attribut à retirer en L2"
)]

use planrs_core::{Project, TaskUid};

use crate::error::CommandError;

/// Allocateur d'UID de tâche d'une session : jamais réutilisés, l'undo ne
/// fait pas reculer le compteur (R-151).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UidAlloc {
    next: u32,
}

impl UidAlloc {
    /// Allocateur d'un projet ouvert. Squelette L0 : compteur à écrire en L2.
    pub fn from_project(p: &Project) -> Self {
        let _ = p;
        Self { next: 0 }
    }

    /// UID suivant. Squelette L0 : `command.not_yet_available`.
    pub fn alloc(&mut self) -> Result<TaskUid, CommandError> {
        let _ = self.next;
        Err(CommandError::not_yet(4))
    }
}
