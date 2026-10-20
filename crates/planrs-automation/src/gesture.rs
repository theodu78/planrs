//! Gestes du Gantt traduits en lots de commandes (J7, ADR-0015 D5 ; N12 de
//! `docs/ui-needs.md` ; règles R-240 à R-243, lot L8).
//!
//! L'interface ne produit qu'un **écart en jours entiers**
//! (`TimeAxis::days_for_dx`), jamais une date (règle 3) : ce module le
//! traduit en lot ordinaire (`task.update`, `progress.update`, `link.add`),
//! appliqué au lâcher comme toute édition (une entrée d'annulation).
//! Pendant le glisser, [`preview`] donne le début et la fin calculés sur le
//! calendrier seul, sans recalcul du planning.
//!
//! Contrat J7 (lot L0) : types et signatures ; squelettes sans lot (aucun
//! geste n'a d'effet) jusqu'au lot L8.

use planrs_core::calendar::CalendarSet;
use planrs_core::{Minutes, Moment, Percent, TaskUid};

use crate::command::Batch;
use crate::error::codes;
use crate::session::Session;

/// Geste sur une barre.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureKind {
    /// Déplacer la barre (R-240).
    Move,
    /// Redimensionner par sa fin (R-241).
    Resize,
    /// Glisser l'avancement (R-242).
    Progress,
}

/// Raison pour laquelle une barre ne se glisse pas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotDraggable {
    /// Récapitulative (seul le lien est permis).
    Summary,
    /// Tâche commencée (arbitrage du 2026-10-20 : le réel ne se déplace pas
    /// d'un geste).
    Started,
    /// Jalon (pas de durée ni d'avancement à glisser).
    Milestone,
    /// Tâche exclue du calcul.
    Excluded,
    /// Ligne vide.
    Blank,
}

/// Geste refusé.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GestureError {
    /// Tâche inconnue.
    #[error("tâche {0} introuvable")]
    UnknownTask(TaskUid),
    /// Barre qui ne se glisse pas ainsi.
    #[error("la tâche {0} ne se glisse pas ainsi ({1:?})")]
    NotDraggable(TaskUid, NotDraggable),
    /// Lien d'une tâche vers elle-même.
    #[error("lien d'une tâche vers elle-même")]
    SameTask,
    /// Écart hors du domaine des dates.
    #[error("écart hors limites")]
    Overflow,
}

impl GestureError {
    /// Code stable (`task.not_found`, `gesture.not_draggable`,
    /// `link.self`, `limit.exceeded`).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnknownTask(_) => codes::TASK_NOT_FOUND,
            Self::NotDraggable(..) => codes::GESTURE_NOT_DRAGGABLE,
            Self::SameTask => codes::LINK_SELF,
            Self::Overflow => codes::LIMIT_EXCEEDED,
        }
    }
}

/// Aperçu d'un geste : dates calculées après application (calendrier seul).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Preview {
    /// Début.
    pub start: Moment,
    /// Fin.
    pub finish: Moment,
    /// Durée (redimensionnement).
    pub duration: Option<Minutes>,
    /// % achevé (glisser l'avancement).
    pub percent: Option<Percent>,
}

/// Tâche planifiée `uid`, sinon l'erreur du geste.
fn scheduled(s: &Session, uid: TaskUid) -> Result<(Moment, Moment), GestureError> {
    if s.project().tasks.get(uid).is_none() {
        return Err(GestureError::UnknownTask(uid));
    }
    s.schedule()
        .scheduled(uid)
        .map(|t| (t.start, t.finish))
        .ok_or(GestureError::NotDraggable(uid, NotDraggable::Excluded))
}

/// Déplace la barre de `uid` de `days` jours calendaires (R-240) ;
/// `days` = 0 : aucun lot.
///
/// # Errors
///
/// [`GestureError`] ; squelette J7 L0 : tâche inconnue ou exclue.
pub fn bar_move(s: &Session, uid: TaskUid, days: i64) -> Result<Option<Batch>, GestureError> {
    scheduled(s, uid)?;
    let _ = days;
    Ok(None)
}

/// Redimensionne la barre de `uid` par sa fin, de `days` jours
/// calendaires (R-241) ; `days` = 0 : aucun lot.
///
/// # Errors
///
/// [`GestureError`] ; squelette J7 L0 : tâche inconnue ou exclue.
pub fn bar_resize(
    s: &Session,
    set: &CalendarSet,
    uid: TaskUid,
    days: i64,
) -> Result<Option<Batch>, GestureError> {
    scheduled(s, uid)?;
    let _ = (set, days);
    Ok(None)
}

/// Pose l'avancement de `uid` au point `days_from_start` jours après son
/// début (R-242).
///
/// # Errors
///
/// [`GestureError`] ; squelette J7 L0 : tâche inconnue ou exclue.
pub fn progress_drag(
    s: &Session,
    set: &CalendarSet,
    uid: TaskUid,
    days_from_start: i64,
) -> Result<Option<Batch>, GestureError> {
    scheduled(s, uid)?;
    let _ = (set, days_from_start);
    Ok(None)
}

/// Lien FS sans décalage de `predecessor` vers `successor` (R-243).
///
/// # Errors
///
/// [`GestureError`] : [`GestureError::SameTask`], tâche inconnue ;
/// squelette J7 L0 : lot vide.
pub fn link_drag(s: &Session, predecessor: TaskUid, successor: TaskUid) -> Result<Batch, GestureError> {
    if predecessor == successor {
        return Err(GestureError::SameTask);
    }
    for uid in [predecessor, successor] {
        if s.project().tasks.get(uid).is_none() {
            return Err(GestureError::UnknownTask(uid));
        }
    }
    Ok(Batch {
        format_version: Batch::FORMAT_VERSION,
        lang: None,
        ops: Vec::new(),
        label: None,
    })
}

/// Aperçu du geste `kind` de `days` jours sur `uid`, sans recalcul.
///
/// # Errors
///
/// [`GestureError`] ; squelette J7 L0 : dates courantes, inchangées.
pub fn preview(
    s: &Session,
    set: &CalendarSet,
    uid: TaskUid,
    kind: GestureKind,
    days: i64,
) -> Result<Preview, GestureError> {
    let (start, finish) = scheduled(s, uid)?;
    let _ = (set, kind, days);
    Ok(Preview {
        start,
        finish,
        duration: None,
        percent: None,
    })
}
