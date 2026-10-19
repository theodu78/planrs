//! Équilibrage durée, travail et unités à l'édition (ADR-0013 D3 ; règles
//! R-170, R-189 à R-194). Contrat J5 (lot L0) ; règles remplies par le lot
//! L2.
//!
//! Fonctions pures, sans calendrier : durées et travail en minutes ouvrées.
//! Les commandes (`task.update`, `assignment.*`) les appellent avec l'état
//! d'une tâche et de ses affectations de travail, puis écrivent le
//! résultat ; le recalcul du planning ne rééquilibre jamais (D3 : l'arrondi
//! dériverait et casserait l'aller-retour).
//!
//! Squelette L0 : [`apply`] contrôle les UID d'affectation désignés et
//! renvoie l'état inchangé ([`Recalculated::Nothing`]).

use crate::duration::{Minutes, Work};
use crate::ids::AssignmentUid;
use crate::model::TaskType;
use crate::quantity::Units;

/// Côté tâche de l'équilibrage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskSide {
    /// Type de tâche (S41).
    pub task_type: TaskType,
    /// Pilotée par l'effort (S42).
    pub effort_driven: bool,
    /// Durée, minutes ouvrées.
    pub duration: Minutes,
    /// Travail total.
    pub work: Work,
    /// Récapitulative (jamais pilotée, R-193).
    pub summary: bool,
}

/// Affectation d'une ressource de travail (les ressources matérielles et
/// de coût n'entrent pas dans l'équilibrage).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Part {
    /// UID de l'affectation.
    pub uid: AssignmentUid,
    /// Unités (1/10 000).
    pub units: Units,
    /// Travail.
    pub work: Work,
}

/// Modification demandée.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edit {
    /// Nouvelle durée de la tâche.
    Duration(Minutes),
    /// Nouveau travail total de la tâche.
    Work(Work),
    /// Nouvelles unités d'une affectation.
    Units {
        /// Affectation.
        uid: AssignmentUid,
        /// Unités.
        units: Units,
    },
    /// Nouveau travail d'une affectation.
    AssignmentWork {
        /// Affectation.
        uid: AssignmentUid,
        /// Travail.
        work: Work,
    },
    /// Ajout d'une affectation (R-190, R-192).
    Add {
        /// UID de l'affectation créée.
        uid: AssignmentUid,
        /// Unités.
        units: Units,
    },
    /// Retrait d'une affectation (R-191, R-192).
    Remove(AssignmentUid),
    /// Changement de type de tâche.
    Type(TaskType),
    /// Changement du pilotage par l'effort.
    EffortDriven(bool),
}

/// Grandeur recalculée par l'équilibrage (matrice S41, R-189).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recalculated {
    /// La durée.
    Duration,
    /// Le travail.
    Work,
    /// Les unités.
    Units,
    /// Rien.
    Nothing,
}

/// État équilibré.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rebalanced {
    /// Durée de la tâche.
    pub duration: Minutes,
    /// Travail total.
    pub work: Work,
    /// Affectations de travail, triées par UID.
    pub parts: Vec<Part>,
    /// Grandeur recalculée.
    pub recalculated: Recalculated,
}

/// Modification refusée.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    /// Tâche à travail fixe non pilotée par l'effort (S41 : « fixed-work
    /// tasks … always effort-driven », R-193).
    #[error("une tâche à travail fixe est toujours pilotée par l'effort")]
    FixedWorkEffortDriven,
    /// Unités nulles sur une tâche à travail fixe (R-181).
    #[error("unités nulles refusées sur une tâche à travail fixe")]
    ZeroUnitsFixedWork,
    /// Durée, travail ou unités d'une récapitulative (R-193).
    #[error("une récapitulative ne s'équilibre pas")]
    SummaryNotEditable,
    /// Affectation désignée absente.
    #[error("affectation {0} inconnue")]
    UnknownAssignment(AssignmentUid),
    /// Résultat hors des bornes (travail, durée, unités).
    #[error("résultat hors des bornes")]
    Overflow,
}

impl EditError {
    /// Code stable.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::FixedWorkEffortDriven => "edit.fixed_work_effort_driven",
            Self::ZeroUnitsFixedWork => "edit.zero_units_fixed_work",
            Self::SummaryNotEditable => "edit.summary_not_editable",
            Self::UnknownAssignment(_) => "edit.unknown_assignment",
            Self::Overflow => "edit.overflow",
        }
    }
}

/// Applique `edit` à la tâche `task` et à ses affectations de travail
/// `parts` (ordre quelconque, UID uniques) : nouvel état, trié par UID.
///
/// # Errors
///
/// [`EditError`] ; squelette L0 : seulement
/// [`EditError::UnknownAssignment`] pour une affectation désignée absente.
pub fn apply(task: &TaskSide, parts: &[Part], edit: &Edit) -> Result<Rebalanced, EditError> {
    let target = match edit {
        Edit::Units { uid, .. } | Edit::AssignmentWork { uid, .. } | Edit::Remove(uid) => Some(*uid),
        Edit::Duration(_) | Edit::Work(_) | Edit::Add { .. } | Edit::Type(_) | Edit::EffortDriven(_) => None,
    };
    if let Some(uid) = target
        && !parts.iter().any(|p| p.uid == uid)
    {
        return Err(EditError::UnknownAssignment(uid));
    }
    let mut parts = parts.to_vec();
    parts.sort_unstable();
    Ok(Rebalanced {
        duration: task.duration,
        work: task.work,
        parts,
        recalculated: Recalculated::Nothing,
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use super::{Edit, EditError, Part, Recalculated, TaskSide, apply};
    use crate::duration::{Minutes, Work};
    use crate::ids::AssignmentUid;
    use crate::model::TaskType;
    use crate::quantity::Units;

    #[test]
    fn contract_apply_skeleton() {
        let task = TaskSide {
            task_type: TaskType::FixedUnits,
            effort_driven: false,
            duration: Minutes::new(480),
            work: Work::new(Minutes::new(960)).unwrap(),
            summary: false,
        };
        let part = |uid, work| Part {
            uid: AssignmentUid::new(uid),
            units: Units::FULL,
            work: Work::new(Minutes::new(work)).unwrap(),
        };
        let parts = [part(5, 480), part(2, 480)];
        let out = apply(&task, &parts, &Edit::EffortDriven(true)).unwrap();
        assert_eq!(out.parts, [part(2, 480), part(5, 480)]);
        assert_eq!(out.recalculated, Recalculated::Nothing);
        let missing = AssignmentUid::new(9);
        assert_eq!(
            apply(&task, &parts, &Edit::Remove(missing)),
            Err(EditError::UnknownAssignment(missing))
        );
        assert_eq!(EditError::Overflow.code(), "edit.overflow");
    }
}
