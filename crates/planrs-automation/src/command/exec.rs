//! Contexte d'exécution d'une commande et répartition (contrat J4, API
//! interne figée pour L4 et L5 ; propriétaire : L2).
//!
//! `Session::apply` exécute chaque opération sur un clone du projet ; une
//! erreur interrompt le lot, l'état de la session reste intact.

use std::collections::BTreeMap;

use planrs_core::{Project, Subject, TaskUid};

use super::{Command, assignment, calendar, constraint, link, project, resource, task};
use crate::error::CommandError;
use crate::outcome::SideEffect;
use crate::text::TextContext;
use crate::uid::UidAlloc;

/// État mutable d'un lot en cours d'application.
pub(crate) struct Ctx<'a> {
    /// Le clone du projet, modifié en place.
    pub project: &'a mut Project,
    /// Allocateur d'UID (R-151), cloné pour le lot.
    pub uids: &'a mut UidAlloc,
    /// Étiquettes `$x` → UID créées dans le lot (R-150).
    pub labels: &'a mut BTreeMap<String, TaskUid>,
    /// Effets de bord, dans l'ordre.
    pub effects: &'a mut Vec<SideEffect>,
    /// Sujet → dernier index d'opération qui l'a touché (R-160).
    pub touched: &'a mut BTreeMap<Subject, usize>,
    /// Index de l'opération courante.
    pub op_index: usize,
    /// Contexte du texte (langue du lot ou de la session).
    #[allow(dead_code, reason = "lu par les commandes de L4 et L5 (durées en texte)")]
    pub text: &'a TextContext,
    /// Tâches créées, dans l'ordre.
    pub created: &'a mut Vec<TaskUid>,
}

/// Exécution d'une commande sur le contexte.
pub(crate) trait Exec {
    /// Applique la commande au clone du projet.
    fn exec(&self, ctx: &mut Ctx<'_>) -> Result<(), CommandError>;
}

impl Exec for Command {
    fn exec(&self, ctx: &mut Ctx<'_>) -> Result<(), CommandError> {
        match self {
            Self::TaskAdd(op) => task::add(op, ctx),
            Self::TaskUpdate(op) => {
                reserved_fields(&op.j5_fields())?;
                task::update(op, ctx)
            }
            Self::TaskDelete(op) => task::delete(op, ctx),
            Self::TaskMove(op) => task::move_to(op, ctx),
            Self::TaskIndent(op) => task::indent(op, ctx),
            Self::TaskOutdent(op) => task::outdent(op, ctx),
            Self::LinkAdd(op) => link::add(op, ctx),
            Self::LinkUpdate(op) => link::update(op, ctx),
            Self::LinkRemove(op) => link::remove(op, ctx),
            Self::TaskSetConstraint(op) => constraint::set_constraint(op, ctx),
            Self::TaskSetDeadline(op) => constraint::set_deadline(op, ctx),
            Self::ProjectSetInfo(op) => {
                reserved_fields(&op.j5_fields())?;
                project::set_info(op, ctx)
            }
            Self::ProjectSetAnchor(op) => project::set_anchor(op, ctx),
            Self::ProjectSetStatusDate(op) => project::set_status_date(op, ctx),
            Self::CalendarSetException(op) => calendar::set_exception(op, ctx),
            Self::CalendarRemoveException(op) => calendar::remove_exception(op, ctx),
            Self::ResourceAdd(op) => resource::add(op, ctx),
            Self::ResourceUpdate(op) => resource::update(op, ctx),
            Self::ResourceDelete(op) => resource::delete(op, ctx),
            Self::AssignmentAdd(op) => assignment::add(op, ctx),
            Self::AssignmentUpdate(op) => assignment::update(op, ctx),
            Self::AssignmentRemove(op) => assignment::remove(op, ctx),
        }
    }
}

/// Contrat J5 : champs étendus de `task.update` et `project.set_info`
/// refusés (`command.not_yet_available`, sur le premier champ posé) tant
/// que le lot L7 ne les exécute pas.
fn reserved_fields(fields: &[&str]) -> Result<(), CommandError> {
    match fields.first() {
        Some(field) => Err(CommandError::not_yet(5).with_field(field)),
        None => Ok(()),
    }
}
