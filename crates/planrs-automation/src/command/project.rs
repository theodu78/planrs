//! Exécution des commandes du projet (informations, ancre, date d'état).
//!
//! Squelette du contrat J4 (L0) : chaque commande répond
//! `command.not_yet_available` jusqu'à son lot (L5).

#![allow(
    dead_code,
    reason = "contrat J4 (L0) : appelé par Session::apply (L2) ; attribut à retirer en L5"
)]

use super::exec::Ctx;
use super::{AnchorSpec, ProjectInfoPatch, SetStatusDate};
use crate::error::CommandError;

/// `set_info` : à écrire en L5.
pub(super) fn set_info(op: &ProjectInfoPatch, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}

/// `set_anchor` : à écrire en L5.
pub(super) fn set_anchor(op: &AnchorSpec, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}

/// `set_status_date` : à écrire en L5.
pub(super) fn set_status_date(op: &SetStatusDate, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}
