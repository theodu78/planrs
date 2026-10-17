//! En-tête à deux niveaux de l'échelle de temps (au-dessus du Gantt).

use egui::Rect;

use super::FrameCtx;
use crate::timescale::TimeAxis;

/// Dessine les deux niveaux de l'en-tête dans `rect`, selon `axis`.
///
/// Corps provisoire : rien n'est dessiné.
pub fn paint_header(_ui: &mut egui::Ui, _fc: &mut FrameCtx<'_>, _axis: &TimeAxis, _rect: Rect) {}
