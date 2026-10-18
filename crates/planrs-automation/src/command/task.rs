//! Exécution des commandes de tâches (R-152 à R-155, R-157, R-158, R-161).
//!
//! Squelette du contrat J4 (L0) : chaque commande répond
//! `command.not_yet_available` jusqu'à son lot (L4).

#![allow(
    dead_code,
    reason = "contrat J4 (L0) : appelé par Session::apply (L2) ; attribut à retirer en L4"
)]

use super::exec::Ctx;
use super::{TaskAdd, TaskDelete, TaskMove, TaskOne, TaskUpdate};
use crate::error::CommandError;

/// `add` : à écrire en L4.
pub(super) fn add(op: &TaskAdd, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}

/// `update` : à écrire en L4.
pub(super) fn update(op: &TaskUpdate, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}

/// `delete` : à écrire en L4.
pub(super) fn delete(op: &TaskDelete, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}

/// `move_to` : à écrire en L4.
pub(super) fn move_to(op: &TaskMove, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}

/// `indent` : à écrire en L4.
pub(super) fn indent(op: &TaskOne, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}

/// `outdent` : à écrire en L4.
pub(super) fn outdent(op: &TaskOne, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}
