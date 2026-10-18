//! Types publics du planning calculé (ADR-0009, spécification J2 § 2.2).
//!
//! Les valeurs calculées vivent ici, jamais dans le modèle (ADR-0008) : un
//! `Schedule` est le résultat d'un appel à [`super::schedule`], immuable.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::calendar::Span;
use crate::diagnostic::Diagnostic;
use crate::duration::Minutes;
use crate::ids::TaskUid;
use crate::model::{ConstraintKind, LinkType};
use crate::quantity::Percent;
use crate::time::Moment;

/// Planning calculé d'un projet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schedule {
    /// Ancre du projet telle que saisie (brute, non recalée).
    pub project_start: Moment,
    /// R-045 : max des fins au plus tôt des tâches actives planifiées ;
    /// `project_start` si rien n'est planifié.
    pub project_finish: Moment,
    /// R-068 : récapitulative du projet (UID 0), si une tâche est planifiée.
    pub project_summary: Option<TaskSchedule>,
    /// Une entrée par tâche du modèle (lignes vides comprises), UID 0 exclu.
    pub tasks: BTreeMap<TaskUid, TaskOutcome>,
    /// R-035 : ordre topologique stable des tâches planifiées (une
    /// récapitulative à la position de son agrégat).
    pub order: Vec<TaskUid>,
    /// Diagnostics (compilation des calendriers comprise), triés,
    /// dédoublonnés.
    pub diagnostics: Vec<Diagnostic>,
}

impl Schedule {
    /// Résultat de la tâche `uid` (UID 0 : aucun, voir
    /// [`Schedule::scheduled`]).
    #[must_use]
    pub fn get(&self, uid: TaskUid) -> Option<&TaskOutcome> {
        self.tasks.get(&uid)
    }

    /// Résultat d'une tâche planifiée ; l'UID 0 donne la récapitulative du
    /// projet.
    #[must_use]
    pub fn scheduled(&self, uid: TaskUid) -> Option<&TaskSchedule> {
        if uid == TaskUid::PROJECT_SUMMARY {
            return self.project_summary.as_ref();
        }
        match self.tasks.get(&uid) {
            Some(TaskOutcome::Scheduled(s)) => Some(s),
            _ => None,
        }
    }

    /// Tâches critiques, dans l'ordre des UID (récapitulative du projet
    /// exclue).
    pub fn critical(&self) -> impl Iterator<Item = (TaskUid, &TaskSchedule)> {
        self.tasks.iter().filter_map(|(uid, o)| match o {
            TaskOutcome::Scheduled(s) if s.critical => Some((*uid, s)),
            _ => None,
        })
    }
}

/// Résultat d'une tâche : planifiée, ou exclue du calcul avec sa raison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TaskOutcome {
    /// Tâche planifiée.
    Scheduled(TaskSchedule),
    /// Tâche exclue du calcul.
    Excluded {
        /// Raison.
        reason: Exclusion,
        /// R-062 : placement d'affichage d'une tâche inactive ; `None` sinon.
        display: Option<Span>,
    },
}

/// Raison d'une exclusion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Exclusion {
    /// Ligne vide (`IsNull`).
    NullRow,
    /// Tâche inactive (R-062).
    Inactive,
    /// Sous une récapitulative inactive (R-062).
    InactiveAncestor,
    /// Tâche manuelle sans début ni fin (R-063).
    ManualUndated,
    /// Récapitulative sans aucun enfant planifié (R-053).
    EmptySummary,
    /// Conversion impossible en passe avant (R-069).
    Unschedulable,
    /// Planification à rebours, non prise en charge avant J3.
    UnsupportedAnchor,
}

/// Valeurs calculées d'une tâche planifiée.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSchedule {
    /// Feuille ou récapitulative.
    pub role: TaskRole,
    /// Tâche en mode manuel (dates saisies conservées, R-063, R-064).
    pub manual: bool,
    /// Début planifié : égal à `early.start` en J2 (distinct en J3 : le plus
    /// tard possible, planification à rebours).
    pub start: Moment,
    /// Fin planifiée : égale à `early.finish` en J2.
    pub finish: Moment,
    /// Dates au plus tôt.
    pub early: Span,
    /// Dates au plus tard.
    pub late: Span,
    /// Feuille : durée de la tâche (manuelle : durée utilisée, R-063) ;
    /// récapitulative : temps ouvré entre son début et sa fin (R-054).
    pub duration: Minutes,
    /// Marge totale, sur l'échelle de marge de la tâche (R-048) ; signée.
    pub total_slack: Minutes,
    /// Marge libre (R-049) ; signée.
    pub free_slack: Minutes,
    /// Critique (R-050).
    pub critical: bool,
    /// Feuille : copie de l'entrée ; récapitulative : R-055.
    pub percent_complete: Percent,
    /// Cause du début au plus tôt (base d'`explain_date`, J6).
    pub driver: Driver,
    /// Dates de repli : une conversion a échoué ici ou en amont (R-069).
    pub degraded: bool,
}

/// Rôle d'une tâche dans la hiérarchie (R-052).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRole {
    /// Tâche sans sous-tâche.
    Leaf,
    /// Tâche récapitulative.
    Summary,
}

/// Cause du début au plus tôt d'une tâche (R-044).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Driver {
    /// Début du projet (R-036).
    ProjectStart,
    /// Borne posée sur une récapitulative ancêtre (R-056).
    SummaryGate {
        /// Récapitulative dont la borne s'applique.
        summary: TaskUid,
    },
    /// Lien depuis un prédécesseur.
    Link {
        /// Prédécesseur (d'origine, éventuellement récapitulatif).
        predecessor: TaskUid,
        /// Type du lien.
        link: LinkType,
    },
    /// Dates saisies (tâche manuelle, R-063).
    Manual,
    /// Agrégat des sous-tâches (récapitulative, R-053).
    Rollup,
    /// Contrainte de date : plancher gagnant ou date imposée (posé par K1,
    /// R-090 à R-104). Le champ ne s'appelle pas `kind`, nom de l'étiquette
    /// serde.
    Constraint {
        /// Type de la contrainte.
        constraint: ConstraintKind,
    },
    /// Placement sur les dates au plus tard (le plus tard possible, posé par
    /// K2, R-105 à R-107).
    LatePlacement,
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use super::Driver;
    use crate::model::ConstraintKind;

    #[test]
    fn driver_constraint_and_late_placement_json() {
        let d = Driver::Constraint {
            constraint: ConstraintKind::FinishNoEarlierThan,
        };
        let json = serde_json::to_string(&d).unwrap();
        assert_eq!(
            json,
            r#"{"kind":"constraint","constraint":"finish_no_earlier_than"}"#
        );
        assert_eq!(serde_json::from_str::<Driver>(&json).unwrap(), d);
        let json = serde_json::to_string(&Driver::LatePlacement).unwrap();
        assert_eq!(json, r#"{"kind":"late_placement"}"#);
        assert_eq!(
            serde_json::from_str::<Driver>(&json).unwrap(),
            Driver::LatePlacement
        );
    }
}
