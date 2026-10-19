//! Delta de diagnostics d'un lot : refus sur nouvelle erreur (R-160, lot
//! L2).
//!
//! Un lot est refusé s'il fait apparaître une erreur absente avant ; les
//! erreurs héritées (fichier importé, état antérieur) ne bloquent pas. Les
//! avertissements et informations nouveaux sont rendus sans bloquer.

use std::collections::{BTreeMap, BTreeSet};

use planrs_core::{Diagnostic, Project, Schedule, Severity, Subject, TaskUid};

/// Diagnostics d'un état : `validate` ∪ diagnostics du planning, triés,
/// dédoublonnés.
pub(crate) fn state_diagnostics(p: &Project, s: &Schedule) -> Vec<Diagnostic> {
    let mut all = p.validate().finish();
    all.extend(s.diagnostics.iter().cloned());
    all.sort();
    all.dedup();
    all
}

/// Nouveautés d'un lot (R-160).
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct DiagDelta {
    /// Erreurs nouvelles (triées) : le lot est refusé si non vide.
    pub errors: Vec<Diagnostic>,
    /// Avertissements et informations absents avant (triés, dédoublonnés).
    pub warnings: Vec<Diagnostic>,
}

/// R-160 : compare les diagnostics avant et après (listes triées).
///
/// Clé d'une erreur : `(code, sujet)` ; pour `link.cycle`, l'ensemble trié
/// des tâches, nouveau seulement s'il n'est inclus dans aucun cycle d'avant
/// (réparer un cycle par étapes). Un avertissement ou une information est
/// nouveau s'il n'est pas présent à l'identique avant.
pub(crate) fn diagnostics_delta(before: &[Diagnostic], after: &[Diagnostic]) -> DiagDelta {
    let mut keys: BTreeSet<(&'static str, Subject)> = BTreeSet::new();
    let mut cycles: Vec<BTreeSet<TaskUid>> = Vec::new();
    for d in before.iter().filter(|d| d.severity() == Severity::Error) {
        match d {
            Diagnostic::LinkCycle { tasks } => cycles.push(tasks.iter().copied().collect()),
            _ => {
                keys.insert((d.code(), d.subject()));
            }
        }
    }
    let mut out = DiagDelta::default();
    for d in after {
        if d.severity() == Severity::Error {
            let new = match d {
                Diagnostic::LinkCycle { tasks } => {
                    !cycles.iter().any(|c| tasks.iter().all(|t| c.contains(t)))
                }
                _ => !keys.contains(&(d.code(), d.subject())),
            };
            if new {
                out.errors.push(d.clone());
            }
        } else if before.binary_search(d).is_err() {
            out.warnings.push(d.clone());
        }
    }
    out.errors.sort();
    out.errors.dedup();
    out.warnings.sort();
    out.warnings.dedup();
    out
}

/// Index d'opération à imputer à une erreur nouvelle (R-160) : la dernière
/// opération qui a touché son sujet (pour `link.cycle` : une tâche du cycle
/// ou un lien entre deux d'entre elles), sinon `None`.
pub(crate) fn blame(d: &Diagnostic, touched: &BTreeMap<Subject, usize>) -> Option<usize> {
    match d {
        Diagnostic::LinkCycle { tasks } => {
            let members: BTreeSet<TaskUid> = tasks.iter().copied().collect();
            touched
                .iter()
                .filter(|(s, _)| match s {
                    Subject::Task(t) => members.contains(t),
                    Subject::Link {
                        predecessor,
                        successor,
                    } => members.contains(predecessor) && members.contains(successor),
                    _ => false,
                })
                .map(|(_, i)| *i)
                .max()
        }
        _ => touched.get(&d.subject()).copied(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use planrs_core::{Diagnostic, Subject, TaskUid};

    use super::{blame, diagnostics_delta};

    fn cycle(uids: &[u32]) -> Diagnostic {
        Diagnostic::LinkCycle {
            tasks: uids.iter().map(|u| TaskUid::new(*u)).collect(),
        }
    }

    #[test]
    fn r160_cycle_subset_not_new() {
        let d = diagnostics_delta(&[cycle(&[5, 7, 9])], &[cycle(&[7, 9])]);
        assert_eq!(d.errors, Vec::new());
        let d = diagnostics_delta(&[cycle(&[5, 7])], &[cycle(&[5, 7, 9])]);
        assert_eq!(d.errors, vec![cycle(&[5, 7, 9])]);
    }

    #[test]
    fn r160_blame_cycle_by_link() {
        let mut touched = BTreeMap::new();
        touched.insert(Subject::Task(TaskUid::new(2)), 0);
        touched.insert(
            Subject::Link {
                predecessor: TaskUid::new(3),
                successor: TaskUid::new(1),
            },
            4,
        );
        assert_eq!(blame(&cycle(&[1, 3]), &touched), Some(4));
        assert_eq!(blame(&cycle(&[5, 6]), &touched), None);
    }
}
