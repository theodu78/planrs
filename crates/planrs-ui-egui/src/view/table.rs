//! Table des tâches dessinée au `Painter` (en-tête et corps virtualisé).
//!
//! Seules les lignes de la plage visible sont dessinées ; chaque ligne déclare
//! un nœud d'accessibilité (`row-label`) pour être retrouvée par les tests.

use std::ops::Range;

use egui::{Pos2, Rect};

use super::FrameCtx;

/// En-tête de la table : titres des colonnes visibles.
///
/// Corps provisoire : rien n'est dessiné.
pub fn paint_header(_ui: &mut egui::Ui, _fc: &mut FrameCtx<'_>, _rect: Rect) {}

/// Corps de la table : lignes `range`, `origin` étant le coin haut-gauche
/// écran de la ligne 0, découpe `clip`.
///
/// Corps provisoire : rien n'est dessiné.
pub fn paint_body(
    _ui: &mut egui::Ui,
    _fc: &mut FrameCtx<'_>,
    _origin: Pos2,
    _range: Range<usize>,
    _clip: Rect,
) {
}
