//! Exécution des exceptions de calendrier (R-165).
//!
//! Squelette du contrat J4 (L0) : chaque commande répond
//! `command.not_yet_available` jusqu'à son lot (L5).

#![allow(
    dead_code,
    reason = "contrat J4 (L0) : appelé par Session::apply (L2) ; attribut à retirer en L5"
)]

use super::exec::Ctx;
use super::{CalendarException, CalendarExceptionKey};
use crate::error::CommandError;

/// `set_exception` : à écrire en L5.
pub(super) fn set_exception(op: &CalendarException, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}

/// `remove_exception` : à écrire en L5.
pub(super) fn remove_exception(op: &CalendarExceptionKey, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(4))
}
