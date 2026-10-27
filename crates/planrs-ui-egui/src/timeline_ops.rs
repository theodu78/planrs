//! Ajout et retrait de tâches de la chronologie (ADR-0017 D4, D7 ;
//! H-UI-79) : un seul `style.apply { patch: { timeline } }` par geste, une
//! entrée d'annulation ; la liste vit dans le style du document
//! (`PlanStyle.timeline`).
//!
//! Lot L0 (contrat) : squelette qui refuse avec « pas encore disponible » ;
//! la bascule et la limite viennent en L3.

use planrs_automation::{Batch, Session};
use planrs_render::style::{MAX_TIMELINE_TASKS, TemplateId};

use crate::action::TimelineOp;
use crate::i18n::TextId;
use crate::selection::Selection;

/// Refus d'une opération de la chronologie (message au bandeau).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimelineOpError {
    /// Plus de [`MAX_TIMELINE_TASKS`] tâches : rien n'est écrit.
    TooMany,
    /// Opération pas encore servie (contrat L0, retiré en L3).
    NotAvailable,
}

impl TimelineOpError {
    /// Message du refus.
    #[must_use]
    pub const fn text_id(self) -> TextId {
        match self {
            Self::TooMany => TextId::TimelineTooMany,
            Self::NotAvailable => TextId::NotAvailableYet,
        }
    }

    /// Plafond cité par le message (`$max`).
    #[must_use]
    pub const fn max() -> usize {
        MAX_TIMELINE_TASKS
    }
}

/// Lot qui applique `op` (`ToggleSelection` ou `Remove`) à la liste de la
/// chronologie du document ; `default` : modèle des documents sans style
/// (préférence, comme la couleur de phase). `Ok(None)` : rien à faire.
///
/// # Errors
///
/// [`TimelineOpError`] : limite dépassée, ou opération pas encore servie.
pub fn batch(
    session: &Session,
    selection: &Selection,
    op: TimelineOp,
    default: TemplateId,
) -> Result<Option<Batch>, TimelineOpError> {
    let _ = (session, selection, op, default);
    Err(TimelineOpError::NotAvailable)
}
