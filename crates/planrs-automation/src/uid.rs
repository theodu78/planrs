//! Allocation des UID de tâche (R-151, lot L2).

use planrs_core::{Project, TaskUid};

use crate::error::{CommandError, CommandErrorKind, codes};

/// Allocateur d'UID de tâche d'une session : jamais réutilisés, l'undo ne
/// fait pas reculer le compteur (R-151).
///
/// `last` est le plus grand UID connu de la session : présent dans le projet
/// ou déjà alloué. Le suivant est `last + 1` par `checked_add` ; 0 (UID de la
/// récapitulative du projet) n'est jamais rendu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UidAlloc {
    last: u32,
}

impl UidAlloc {
    /// Allocateur d'un projet ouvert : `last` = plus grand UID de tâche
    /// présent (0 si aucune tâche).
    pub fn from_project(p: &Project) -> Self {
        Self {
            last: max_task_uid(p),
        }
    }

    /// UID suivant (R-151).
    ///
    /// # Errors
    ///
    /// `limit.exceeded { what: "task_uid" }` si `last` vaut `u32::MAX`.
    pub fn alloc(&mut self) -> Result<TaskUid, CommandError> {
        let next = self.last.checked_add(1).ok_or_else(|| {
            CommandError::new(
                codes::LIMIT_EXCEEDED,
                CommandErrorKind::Limit {
                    what: "task_uid",
                    max: u64::from(u32::MAX),
                },
            )
        })?;
        self.last = next;
        Ok(TaskUid::new(next))
    }

    /// Prend en compte les UID présents dans `p` (`last = max(last, max
    /// présent)`) : un UID posé hors de l'allocateur n'est jamais rendu.
    pub fn observe(&mut self, p: &Project) {
        self.last = self.last.max(max_task_uid(p));
    }
}

/// Plus grand UID de tâche présent, 0 si aucune tâche.
fn max_task_uid(p: &Project) -> u32 {
    p.tasks.max_uid().map_or(0, TaskUid::get)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use planrs_core::{Moment, Project, ProjectInit, ScheduleAnchor, Task, TaskUid};

    use super::UidAlloc;
    use crate::error::codes;

    fn project(uids: &[u32]) -> Project {
        let start: Moment = "2026-11-02T08:00".parse().unwrap();
        let mut p = Project::new(ProjectInit {
            name: "P".to_owned(),
            anchor: ScheduleAnchor::FromStart(start),
            current_date: start,
            calendar_names: planrs_core::BuiltinCalendarNames::default(),
        });
        for &u in uids {
            let t = Task::new(TaskUid::new(u), format!("T{u}"), &p.options);
            p.tasks.push(t).unwrap();
        }
        p
    }

    #[test]
    fn r151_alloc_after_max_present() {
        let mut a = UidAlloc::from_project(&project(&[1, 2, 7]));
        assert_eq!(a.alloc().unwrap(), TaskUid::new(8));
        assert_eq!(a.alloc().unwrap(), TaskUid::new(9));
    }

    #[test]
    fn r151_empty_project_starts_at_one() {
        let mut a = UidAlloc::from_project(&project(&[]));
        assert_eq!(a.alloc().unwrap(), TaskUid::new(1));
    }

    #[test]
    fn r151_observe_never_goes_back() {
        let mut a = UidAlloc::from_project(&project(&[3]));
        assert_eq!(a.alloc().unwrap(), TaskUid::new(4));
        a.observe(&project(&[1]));
        assert_eq!(a.alloc().unwrap(), TaskUid::new(5));
        a.observe(&project(&[40]));
        assert_eq!(a.alloc().unwrap(), TaskUid::new(41));
    }

    #[test]
    fn r151_overflow() {
        let mut a = UidAlloc::from_project(&project(&[u32::MAX]));
        assert_eq!(a.alloc().unwrap_err().code, codes::LIMIT_EXCEEDED);
        // Refus sans effet : le compteur ne bouge pas.
        assert_eq!(a, UidAlloc { last: u32::MAX });
    }
}
