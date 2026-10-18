//! Exécution des contraintes et échéances (R-159).
//!
//! Squelette du contrat J4 (L0) : chaque commande répond
//! `command.not_yet_available` jusqu'à son lot (L5).

#![allow(
    dead_code,
    reason = "contrat J4 (L0) : appelé par Session::apply (L2) ; attribut à retirer en L5"
)]

use super::exec::Ctx;
use super::{SetConstraint, SetDeadline};
use crate::error::CommandError;

/// `set_constraint` : à écrire en L5.
pub(super) fn set_constraint(op: &SetConstraint, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}

/// `set_deadline` : à écrire en L5.
pub(super) fn set_deadline(op: &SetDeadline, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}
