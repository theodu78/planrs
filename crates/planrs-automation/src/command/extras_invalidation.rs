//! Invalidation des `Extras` d'une entité modifiée (R-161) : table des clés
//! et jet, partagés par les commandes de tâches, de liens, de contraintes et
//! de calendriers.
//!
//! Les `Extras` gardent tels quels des éléments MSPDI hors modèle. Certains
//! dépendent des dates calculées par MS Project (répartition dans le temps,
//! nivellement, interruption) : ils deviennent faux dès qu'une commande
//! change ce dont ces dates dépendent, et sont alors jetés, chaque jet
//! signalé par `SideEffect::ExtrasDropped`. Table figée par un test contre
//! `planrs_mspdi::schema`.

use planrs_core::{Subject, TaskUid};
use planrs_mspdi::extras::ORIGIN;

use super::exec::Ctx;
use crate::outcome::SideEffect;

/// Champ (ou groupe de champs) d'une tâche modifié par une commande.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TaskChange {
    /// Nom.
    Name,
    /// Notes.
    Notes,
    /// Durée.
    Duration,
    /// Contrainte (début ou fin saisis, R-158).
    Constraint,
    /// Drapeau jalon.
    Milestone,
    /// Tâche active.
    Active,
    /// Liens de la tâche (prédécesseurs ajoutés, modifiés ou retirés).
    Links,
    /// Place dans la hiérarchie (niveau, parent, récapitulative ou non).
    Outline,
}

/// Éléments de tâche MSPDI qui dépendent des dates calculées : répartition
/// (`TimephasedData`), nivellement (`LevelingDelay`, `LevelingDelayFormat`,
/// `PreLeveledStart`, `PreLeveledFinish`), interruption (`Resume`, `Stop`).
pub(crate) const TASK_SCHEDULE_KEYS: &[&str] = &[
    "TimephasedData",
    "LevelingDelay",
    "LevelingDelayFormat",
    "PreLeveledStart",
    "PreLeveledFinish",
    "Resume",
    "Stop",
];

/// Éléments d'affectation MSPDI jetés quand la durée de sa tâche change ou
/// qu'un calendrier qui la planifie change (R-165) : répartition.
pub(crate) const ASSIGNMENT_SCHEDULE_KEYS: &[&str] = &["TimephasedData"];

/// R-161 : clés d'`Extras` de la tâche jetées quand `change` est modifié.
pub(crate) const fn task_keys(change: TaskChange) -> &'static [&'static str] {
    match change {
        TaskChange::Name | TaskChange::Notes => &[],
        TaskChange::Duration
        | TaskChange::Constraint
        | TaskChange::Milestone
        | TaskChange::Active
        | TaskChange::Links
        | TaskChange::Outline => TASK_SCHEDULE_KEYS,
    }
}

/// Retire de `extras` les entrées MSPDI dont la clé est dans `keys` ; rend
/// les clés retirées, triées, sans doublon. Seule implémentation du jet
/// (tâches, affectations, calendriers).
pub(super) fn drop_keys(extras: &mut planrs_core::Extras, keys: &[&str]) -> Vec<String> {
    let mut dropped = Vec::new();
    extras.0.retain(|e| {
        let hit = e.origin == ORIGIN && keys.contains(&e.key.as_str());
        if hit {
            dropped.push(e.key.clone());
        }
        !hit
    });
    dropped.sort();
    dropped.dedup();
    dropped
}

impl Ctx<'_> {
    /// R-161 : jette les `Extras` de la tâche `uid` que `change` invalide
    /// et signale le jet ; sans effet si rien n'est jeté.
    pub(crate) fn drop_task_extras(&mut self, uid: TaskUid, change: TaskChange) {
        let keys = task_keys(change);
        if keys.is_empty() {
            return;
        }
        let Some(task) = self.project.tasks.get_mut(uid) else {
            return;
        };
        let dropped = drop_keys(&mut task.extras, keys);
        if !dropped.is_empty() {
            self.effects.push(SideEffect::ExtrasDropped {
                subject: Subject::Task(uid),
                keys: dropped,
            });
        }
    }

    /// R-161 : la durée de la tâche `uid` a changé ; jette la répartition
    /// de chacune de ses affectations (un effet par affectation touchée).
    pub(crate) fn drop_assignment_extras(&mut self, uid: TaskUid) {
        let touched: Vec<_> = self
            .project
            .assignments
            .iter()
            .filter(|a| a.task == uid && !a.extras.is_empty())
            .map(planrs_core::HasUid::uid)
            .collect();
        for a in touched {
            let Some(assignment) = self.project.assignments.get_mut(a) else {
                continue;
            };
            let dropped = drop_keys(&mut assignment.extras, ASSIGNMENT_SCHEDULE_KEYS);
            if !dropped.is_empty() {
                self.effects.push(SideEffect::ExtrasDropped {
                    subject: Subject::Assignment(a),
                    keys: dropped,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use planrs_mspdi::schema::{Parent, sequence};

    use super::{ASSIGNMENT_SCHEDULE_KEYS, TASK_SCHEDULE_KEYS, TaskChange, task_keys};

    #[test]
    fn r161_table_matches_mspdi_schema() {
        // Chaque clé jetée est un élément de la séquence MSPDI de son
        // entité : une faute de frappe ne jetterait jamais rien.
        for key in TASK_SCHEDULE_KEYS {
            assert!(sequence(Parent::Task).slot(key).is_some(), "Task/{key}");
        }
        for key in ASSIGNMENT_SCHEDULE_KEYS {
            assert!(
                sequence(Parent::Assignment).slot(key).is_some(),
                "Assignment/{key}"
            );
        }
        // Table figée.
        assert_eq!(task_keys(TaskChange::Name), [] as [&str; 0]);
        assert_eq!(task_keys(TaskChange::Notes), [] as [&str; 0]);
        for c in [
            TaskChange::Duration,
            TaskChange::Constraint,
            TaskChange::Milestone,
            TaskChange::Active,
            TaskChange::Links,
            TaskChange::Outline,
        ] {
            assert_eq!(task_keys(c), TASK_SCHEDULE_KEYS, "{c:?}");
        }
        assert_eq!(
            TASK_SCHEDULE_KEYS,
            [
                "TimephasedData",
                "LevelingDelay",
                "LevelingDelayFormat",
                "PreLeveledStart",
                "PreLeveledFinish",
                "Resume",
                "Stop"
            ]
        );
        assert_eq!(ASSIGNMENT_SCHEDULE_KEYS, ["TimephasedData"]);
    }
}
