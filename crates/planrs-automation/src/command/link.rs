//! Exécution des commandes de liens (R-156, R-161).
//!
//! Squelette du contrat J4 (L0) : chaque commande répond
//! `command.not_yet_available` jusqu'à son lot (L5).

#![allow(
    dead_code,
    reason = "contrat J4 (L0) : appelé par Session::apply (L2) ; attribut à retirer en L5"
)]

use super::exec::Ctx;
use super::{LinkKey, LinkSpec};
use crate::error::CommandError;

/// `add` : à écrire en L5.
pub(super) fn add(op: &LinkSpec, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}

/// `update` : à écrire en L5.
pub(super) fn update(op: &LinkSpec, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}

/// `remove` : à écrire en L5.
pub(super) fn remove(op: &LinkKey, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}
