//! Commandes `resource.*` (R-195 ; contrat J5, lot L0). Exécution écrite
//! par le lot L7 ; les squelettes refusent `command.not_yet_available`.

use super::exec::Ctx;
use super::{ResourceAdd, ResourceDelete, ResourceUpdate};
use crate::error::CommandError;

/// `resource.add` (R-195).
pub(super) fn add(op: &ResourceAdd, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(5))
}

/// `resource.update` (R-195).
pub(super) fn update(op: &ResourceUpdate, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(5))
}

/// `resource.delete` (R-195).
pub(super) fn delete(op: &ResourceDelete, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let _ = (op, ctx);
    Err(CommandError::not_yet(5))
}
