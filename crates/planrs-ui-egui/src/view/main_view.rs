//! Vue principale : en-têtes, séparateur, `ScrollArea` verticale unique pour
//! la table et le Gantt (alignés dans la même image), défilement horizontal
//! du Gantt.

use super::{FrameCtx, ViewTransient};

/// Affiche la vue principale dans la place restante de `ui`.
///
/// Corps provisoire : rien n'est dessiné.
pub fn show(_ui: &mut egui::Ui, _fc: &mut FrameCtx<'_>, _view: &mut ViewTransient) {}
