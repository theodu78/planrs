//! Équilibrage d'une saisie d'avancement (ADR-0014 D3, D4 ; règles R-200
//! (équation), R-201, R-202, R-211 à R-213). Contrat J6 (lot L0) ; règles
//! remplies par le lot L2.
//!
//! Fonction pure, sans planning ni projet complet : la tâche, ses
//! affectations de travail, la saisie et un contexte (dates planifiées
//! courantes, calendrier de mesure, date d'état, options). La commande
//! `progress.update` (R-216) l'appelle puis écrit le résultat ; le recalcul
//! ne rééquilibre jamais.
//!
//! Squelette L0 : [`apply`] refuse une récapitulative et une tâche inactive
//! (R-213) et renvoie sinon l'état inchangé.

use crate::calendar::{CompiledCalendar, Span};
use crate::duration::{Minutes, Work};
use crate::ids::AssignmentUid;
use crate::model::{Progress, ProgressOption, ProgressOptions, Task};
use crate::quantity::Percent;
use crate::time::Moment;

/// Une saisie d'avancement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    /// % achevé (durée), 1/10 000 (R-201).
    Percent(Percent),
    /// Début réel (R-202).
    ActualStart(Moment),
    /// Fin réelle (R-202).
    ActualFinish(Moment),
    /// Durée réelle, minutes ouvrées (R-202).
    ActualDuration(Minutes),
    /// Durée restante, minutes ouvrées (R-202 : D = réel + reste).
    RemainingDuration(Minutes),
    /// % travail achevé, 1/10 000 (R-211).
    PercentWork(Percent),
    /// Travail réel de la tâche, réparti sur ses affectations (R-211).
    ActualWork(Work),
    /// % physique, 1/10 000 (R-212, sans effet sur les dates).
    Physical(Percent),
}

/// Contexte d'une saisie.
#[derive(Clone, Copy, Debug)]
pub struct Ctx<'a> {
    /// Dates planifiées courantes de la tâche (début réel implicite, fin
    /// réelle à 100 %, R-201).
    pub scheduled: Span,
    /// Calendrier de mesure de la tâche (R-113) : durée entre le début et
    /// la fin réels (R-202).
    pub measure: &'a CompiledCalendar,
    /// Date d'état effective (R-034, R-219).
    pub status_date: Moment,
    /// Options de l'avancement (R-218).
    pub options: ProgressOptions,
    /// Tâche récapitulative (refusée, R-213) : la hiérarchie se lit sur le
    /// projet, pas sur la tâche seule (ajout au contrat de `j6.md`).
    pub summary: bool,
}

/// Affectation de travail d'une tâche (les ressources matérielles et de
/// coût n'ont pas de travail réel).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Part {
    /// UID de l'affectation.
    pub uid: AssignmentUid,
    /// Travail.
    pub work: Work,
    /// Travail réel.
    pub actual_work: Work,
}

/// Remarque d'une saisie acceptée.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EditNote {
    /// Jalon à un % strictement entre 0 et 100 : ramené à 100 %
    /// (`progress.milestone_rounded`, R-213).
    MilestoneRounded,
    /// Option que la saisie ne suit pas (`progress.option_unsupported`,
    /// R-218).
    OptionUnsupported(ProgressOption),
}

/// Saisie équilibrée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edited {
    /// Avancement à écrire.
    pub progress: Progress,
    /// Durée de la tâche (minutes ouvrées) ; change si la durée réelle
    /// dépasse D, si un reste est saisi ou si la fin réelle mesure D
    /// (R-202).
    pub duration: Minutes,
    /// Affectations de travail, triées par UID (R-211).
    pub parts: Vec<Part>,
    /// Remarques, dans l'ordre.
    pub notes: Vec<EditNote>,
}

/// Saisie refusée.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EditError {
    /// Récapitulative, automatique ou manuelle (R-213,
    /// `progress.summary_not_editable`).
    #[error("l'avancement d'une récapitulative ne se saisit pas")]
    SummaryNotEditable,
    /// Tâche inactive (R-213, `progress.inactive`).
    #[error("l'avancement d'une tâche inactive ne se saisit pas")]
    Inactive,
    /// Fin réelle avant le début réel (R-202, `progress.invalid_order`).
    #[error("la fin réelle précède le début réel")]
    InvalidOrder,
    /// Deux champs qui s'excluent (R-202, `param.conflict`), nommés comme
    /// dans `progress.update`.
    #[error("« {0} » et « {1} » ne se saisissent pas ensemble")]
    Conflict(&'static str, &'static str),
    /// Résultat hors des bornes (durée, travail).
    #[error("résultat hors des bornes")]
    Overflow,
}

impl EditError {
    /// Code stable (celui de l'erreur de commande, R-216).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::SummaryNotEditable => "progress.summary_not_editable",
            Self::Inactive => "progress.inactive",
            Self::InvalidOrder => "progress.invalid_order",
            Self::Conflict(..) => "param.conflict",
            Self::Overflow => "progress.overflow",
        }
    }
}

/// Applique la saisie `inputs` à `task` et à ses affectations de travail
/// `parts` (ordre quelconque, UID uniques), dans l'ordre de R-202 (début
/// réel, durée réelle, reste, %, fin réelle, puis travail).
///
/// # Errors
///
/// [`EditError`] ; squelette L0 : seulement
/// [`EditError::SummaryNotEditable`] et [`EditError::Inactive`].
pub fn apply(task: &Task, parts: &[Part], inputs: &[Input], ctx: &Ctx<'_>) -> Result<Edited, EditError> {
    if ctx.summary {
        return Err(EditError::SummaryNotEditable);
    }
    if !task.active {
        return Err(EditError::Inactive);
    }
    let _ = inputs;
    let mut parts = parts.to_vec();
    parts.sort_unstable();
    Ok(Edited {
        progress: task.progress,
        duration: task.duration.minutes(),
        parts,
        notes: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "tests : les interdits de la règle 10 ne s'appliquent pas, un échec doit paniquer"
    )]

    use super::{Ctx, EditError, Input, Part, apply};
    use crate::arena::Arena;
    use crate::calendar::{CompiledCalendar, Span, standard};
    use crate::duration::{Minutes, Work};
    use crate::ids::{AssignmentUid, CalendarUid, TaskUid};
    use crate::model::{ProgressOptions, ProjectOptions, Task};
    use crate::quantity::Percent;

    #[test]
    fn skeleton_refuses_summary_and_inactive() {
        let u = CalendarUid::new(1);
        let cals = Arena::try_from(vec![standard(u, "S".to_owned())]).unwrap();
        let cal = CompiledCalendar::compile(&cals, u).unwrap();
        let at = |s: &str| s.parse().unwrap();
        let mut ctx = Ctx {
            scheduled: Span {
                start: at("2026-10-19T08:00"),
                finish: at("2026-10-23T17:00"),
            },
            measure: &cal,
            status_date: at("2026-10-19T08:00"),
            options: ProgressOptions::default(),
            summary: false,
        };
        let mut t = Task::new(TaskUid::new(1), "T".to_owned(), &ProjectOptions::default());
        let parts = [
            Part {
                uid: AssignmentUid::new(2),
                work: Work::ZERO,
                actual_work: Work::ZERO,
            },
            Part {
                uid: AssignmentUid::new(1),
                work: Work::ZERO,
                actual_work: Work::ZERO,
            },
        ];
        let e = apply(&t, &parts, &[Input::Percent(Percent::FULL)], &ctx).unwrap();
        assert_eq!(e.progress, t.progress);
        assert_eq!(e.duration, Minutes::new(480));
        assert_eq!(e.parts.first().map(|p| p.uid), Some(AssignmentUid::new(1)));
        t.active = false;
        assert_eq!(apply(&t, &parts, &[], &ctx), Err(EditError::Inactive));
        ctx.summary = true;
        assert_eq!(apply(&t, &parts, &[], &ctx), Err(EditError::SummaryNotEditable));
        assert_eq!(EditError::Conflict("a", "b").code(), "param.conflict");
    }
}
