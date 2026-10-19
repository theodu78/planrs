//! Exécution des commandes du projet (informations, ancre, date d'état).
//!
//! Aucune n'est une règle de calcul : elles posent des entrées du moteur
//! (sens et date pilote, date d'état), que le recalcul du lot exploite.

use planrs_core::{ScheduleAnchor, Subject};

use super::exec::Ctx;
use super::{AnchorSpec, Patch, ProjectInfoPatch, SetStatusDate};
use crate::error::CommandError;

/// Applique un `Patch<String>` : `Clear` vide le champ.
fn patch_string(field: &mut String, patch: &Patch<String>) {
    match patch {
        Patch::Keep => {}
        Patch::Set(v) => field.clone_from(v),
        Patch::Clear => field.clear(),
    }
}

/// `project.set_info` : champs descriptifs ; absent gardé, `null` vidé.
#[allow(
    clippy::unnecessary_wraps,
    reason = "signature commune des commandes, appelée par la répartition `Exec` (exec.rs)"
)]
pub(super) fn set_info(op: &ProjectInfoPatch, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    let info = &mut ctx.project.info;
    patch_string(&mut info.name, &op.name);
    patch_string(&mut info.title, &op.title);
    patch_string(&mut info.subject, &op.subject);
    patch_string(&mut info.author, &op.author);
    patch_string(&mut info.manager, &op.manager);
    patch_string(&mut info.company, &op.company);
    patch_string(&mut info.category, &op.category);
    ctx.touched.insert(Subject::Project, ctx.op_index);
    Ok(())
}

/// `project.set_anchor` : sens de planification et date pilote. Les
/// contraintes des tâches existantes ne changent pas (ADR-0012, lot L5) :
/// seules les tâches ajoutées ensuite prennent la contrainte par défaut du
/// nouveau sens (R-108, R-152).
#[allow(
    clippy::unnecessary_wraps,
    reason = "signature commune des commandes, appelée par la répartition `Exec` (exec.rs)"
)]
pub(super) fn set_anchor(op: &AnchorSpec, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    ctx.project.anchor = match *op {
        AnchorSpec::Start(m) => ScheduleAnchor::FromStart(m),
        AnchorSpec::Finish(m) => ScheduleAnchor::FromFinish(m),
    };
    ctx.touched.insert(Subject::Project, ctx.op_index);
    Ok(())
}

/// `project.set_status_date` : pose (`Set`), retire (`Clear` : « NA », la
/// date du jour s'applique, R-034) ou garde la date d'état.
#[allow(
    clippy::unnecessary_wraps,
    reason = "signature commune des commandes, appelée par la répartition `Exec` (exec.rs)"
)]
pub(super) fn set_status_date(op: &SetStatusDate, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
    match op.date {
        Patch::Keep => {}
        Patch::Set(d) => ctx.project.status_date = Some(d),
        Patch::Clear => ctx.project.status_date = None,
    }
    ctx.touched.insert(Subject::Project, ctx.op_index);
    Ok(())
}
