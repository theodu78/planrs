//! Commandes `assignment.*` (R-196 ; contrat J5, lot L0). Exécution écrite
//! par le lot L7 ; les squelettes refusent `command.not_yet_available`.

use super::exec::Ctx;
use super::{AssignmentAdd, AssignmentKey, AssignmentUpdate};
use crate::error::CommandError;

/// `assignment.add` (R-196).
pub(super) fn add(op: &AssignmentAdd, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(5))
}

/// `assignment.update` (R-196).
pub(super) fn update(op: &AssignmentUpdate, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(5))
}

/// `assignment.remove` (R-196).
pub(super) fn remove(op: &AssignmentKey, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(5))
}
