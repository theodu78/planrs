//! Explication d'une date (J6, ADR-0014 D8 ; règles R-230 à R-233).
//! Contrat J6 (lot L0) ; règles remplies par le lot L7.
//!
//! [`explain`] lit un `Schedule` déjà calculé : aucun recalcul, aucune
//! trace gardée par le moteur. La passe avant ne pose qu'une valeur de plus,
//! `TaskSchedule::driver_bound` (borne brute du pilote, R-230). Les étapes
//! citent le pilote, la borne brute, le recalage sur le calendrier, le
//! retard de nivellement, l'avancement (réel, reprise) et la durée ; la
//! chaîne remonte les pilotes, bornée en profondeur (règle 10). Aucune date
//! n'est rédigée hors du moteur (règle 3) : les résumés FR/EN sont des
//! gabarits remplis par ces champs (R-233).
//!
//! Squelette L0 : contrôles d'entrée, première étape (pilote et borne
//! brute) et côté au plus tard ; ni recalage, ni chaîne.

use serde::Serialize;

use super::types::{Driver, LateDriver, Schedule};
use crate::calendar::Span;
use crate::duration::Minutes;
use crate::ids::{AssignmentUid, CalendarUid, TaskUid};
use crate::model::{ConstraintKind, LevelingDelay, LinkType, Project};
use crate::time::Moment;

/// Profondeur de chaîne par défaut (R-232).
pub const DEFAULT_DEPTH: u8 = 5;
/// Profondeur de chaîne maximale (R-232, règle 10).
pub const MAX_DEPTH: u8 = 50;

/// Date expliquée.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Début planifié (R-230).
    #[default]
    Start,
    /// Fin planifiée (R-231).
    Finish,
}

/// Étape d'une explication, dans l'ordre de R-230 et R-231.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum Step {
    /// Pilote du début au plus tôt et sa borne brute
    /// (`TaskSchedule::driver_bound`).
    Driver {
        /// Pilote (R-044).
        driver: Driver,
        /// Borne brute ; `None` pour `Manual`, `Rollup`, `Actual`.
        raw: Option<Moment>,
    },
    /// Recalage de la borne brute sur le calendrier de placement (R-024,
    /// R-030).
    Snap {
        /// Borne brute.
        raw: Moment,
        /// Instant recalé.
        snapped: Moment,
        /// Calendrier de placement ; `None` en temps écoulé.
        calendar: Option<CalendarUid>,
    },
    /// Retard de nivellement enregistré (R-178).
    LevelingDelay {
        /// Retard.
        delay: LevelingDelay,
        /// Début au plus tôt non retardé.
        from: Moment,
        /// Début retardé.
        to: Moment,
    },
    /// Dates réelles (R-203, R-204).
    Actual {
        /// Début réel.
        start: Moment,
        /// Fin réelle (tâche achevée).
        finish: Option<Moment>,
    },
    /// Reprise du reste d'une tâche en cours (R-204, R-205).
    Resume {
        /// Fin de la partie réelle.
        stop: Moment,
        /// Reprise.
        resume: Moment,
        /// Cause de la reprise.
        cause: ResumeCause,
    },
    /// Durée placée (R-231).
    Duration {
        /// Minutes de la durée (ou du reste d'une tâche en cours).
        minutes: Minutes,
        /// Calendrier de placement ; `None` en temps écoulé.
        calendar: Option<CalendarUid>,
        /// Feuille à parts : affectation la plus longue (R-173).
        longest: Option<AssignmentUid>,
    },
    /// Contrainte de date qui ne lie pas (R-230).
    ConstraintNotBinding {
        /// Type de la contrainte.
        kind: ConstraintKind,
        /// Date de la contrainte.
        date: Moment,
    },
    /// Dates de repli (R-069) : code stable de la cause.
    Degraded {
        /// Code stable.
        code: &'static str,
    },
}

/// Cause de la reprise d'une tâche en cours (R-204, R-205).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ResumeCause {
    /// Premier instant ouvré après la fin de la partie réelle.
    NextWorking,
    /// Lien depuis un prédécesseur.
    Link {
        /// Prédécesseur.
        predecessor: TaskUid,
        /// Type du lien.
        link: LinkType,
    },
    /// Contrainte de début (SNET, MSO).
    Constraint(ConstraintKind),
    /// Reprise saisie (`Progress::resume`).
    Entered,
}

/// Côté au plus tard d'une tâche (R-231).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct LateState {
    /// Dates au plus tard.
    pub late: Span,
    /// Cause de la fin au plus tard (R-047).
    pub driver: LateDriver,
    /// Marge totale (R-048).
    pub total_slack: Minutes,
    /// Marge libre (R-049).
    pub free_slack: Minutes,
    /// Critique (R-050).
    pub critical: bool,
}

/// Maillon de la chaîne des pilotes (R-232).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ChainLink {
    /// Tâche.
    pub uid: TaskUid,
    /// Début planifié.
    pub start: Moment,
    /// Fin planifiée.
    pub finish: Moment,
    /// Pilote de son début.
    pub driver: Driver,
    /// Borne brute du pilote.
    pub raw: Option<Moment>,
}

/// Explication d'une date.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Explanation {
    /// Tâche expliquée.
    pub task: TaskUid,
    /// Date expliquée.
    pub side: Side,
    /// Valeur de la date (celle du `Schedule`).
    pub date: Moment,
    /// Étapes, dans l'ordre.
    pub steps: Vec<Step>,
    /// Côté au plus tard.
    pub late: LateState,
    /// Chaîne des pilotes, de la tâche vers l'amont (R-232).
    pub chain: Vec<ChainLink>,
    /// Chaîne coupée à la profondeur demandée.
    pub truncated: bool,
    /// L'explication repose sur une hypothèse non vérifiée (R-206 : marge
    /// d'une tâche en cours).
    pub hypothesis: bool,
}

/// Explication impossible.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ExplainError {
    /// Tâche inexistante (`task.not_found`).
    #[error("aucune tâche d'UID {0}")]
    UnknownTask(TaskUid),
    /// Tâche exclue du calcul : aucune date à expliquer
    /// (`explain.not_scheduled`).
    #[error("la tâche {0} n'est pas planifiée")]
    NotScheduled(TaskUid),
    /// Profondeur hors de `1..=50` (`param.invalid`, R-232).
    #[error("profondeur {0} hors de 1 à 50")]
    DepthOutOfRange(u8),
}

/// Explique la date `side` de la tâche `uid` (UID 0 : récapitulative du
/// projet) dans le planning `s` de `p`, chaîne bornée à `depth` maillons
/// (1 à [`MAX_DEPTH`]).
///
/// # Errors
///
/// [`ExplainError`].
pub fn explain(
    p: &Project,
    s: &Schedule,
    uid: TaskUid,
    side: Side,
    depth: u8,
) -> Result<Explanation, ExplainError> {
    if uid != TaskUid::PROJECT_SUMMARY && p.tasks.get(uid).is_none() {
        return Err(ExplainError::UnknownTask(uid));
    }
    if !(1..=MAX_DEPTH).contains(&depth) {
        return Err(ExplainError::DepthOutOfRange(depth));
    }
    let ts = s.scheduled(uid).ok_or(ExplainError::NotScheduled(uid))?;
    Ok(Explanation {
        task: uid,
        side,
        date: match side {
            Side::Start => ts.start,
            Side::Finish => ts.finish,
        },
        steps: vec![Step::Driver {
            driver: ts.driver,
            raw: ts.driver_bound,
        }],
        late: LateState {
            late: ts.late,
            driver: ts.late_driver,
            total_slack: ts.total_slack,
            free_slack: ts.free_slack,
            critical: ts.critical,
        },
        chain: Vec::new(),
        truncated: false,
        hypothesis: false,
    })
}
